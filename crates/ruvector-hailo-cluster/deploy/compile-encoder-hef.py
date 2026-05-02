#!/usr/bin/env python3
"""Compile the encoder-only ONNX (iter 139) to Hailo-8 .hef.

Companion to compile-hef.py. Uses the encoder-only export from
export-minilm-encoder-onnx.py — no Gather/Where/Expand ops, just clean
MatMul/Softmax/Add/Mul/Reshape encoder primitives that Hailo can fuse.

If this compile succeeds, the HEF surgery in ADR-167 is unblocked.
The host-side embedding lookup + mask construction will be wired in
HailoEmbedder in a follow-up iter.

Usage: python3 compile-encoder-hef.py <encoder_onnx> <out_hef>
"""

import os
import sys
from pathlib import Path

os.environ.setdefault("TRANSFORMERS_NO_TF", "1")

from hailo_sdk_client import ClientRunner
import numpy as np

HW_ARCH = "hailo8"
NET_NAME = "minilm_encoder"
SEQ_LEN = 128
HIDDEN = 384


def main(onnx_path: str, out_hef: str) -> None:
    onnx_path = Path(onnx_path).resolve()
    out_hef = Path(out_hef).resolve()
    work = out_hef.parent

    print(f"==> [parse] {onnx_path}", flush=True)
    runner = ClientRunner(hw_arch=HW_ARCH)
    runner.translate_onnx_model(
        str(onnx_path),
        net_name=NET_NAME,
        start_node_names=["hidden_states"],
        end_node_names=["last_hidden_state"],
        net_input_shapes={
            "hidden_states": [1, SEQ_LEN, HIDDEN],
        },
    )

    parsed_har = work / f"{NET_NAME}_parsed.har"
    runner.save_har(str(parsed_har))
    print(f"    parsed HAR → {parsed_har}", flush=True)

    print("==> [optimize] random calibration set (FP→INT8)", flush=True)
    # Iter 142: root-cause analysis of the iter-139 KeyError. The
    # SDK's stats_collection._get_build_inputs() returns a dict keyed
    # by the dataset keys ("hidden_states") but hailo_model.build()
    # iterates over self.flow.input_nodes (internal layer names like
    # "minilm_encoder/input_layer1") and looks them up in the dict.
    # KeyError follows. Workaround: key the calibration dataset by the
    # internal input layer name instead of the ONNX input name.
    runner.load_model_script("model_optimization_flavor(optimization_level=0)\n")

    rng = np.random.default_rng(seed=42)
    # Discover the actual input layer name from the parsed network.
    input_layer_name = None
    try:
        hn = runner.get_hn()
        import json as _json
        hn_d = _json.loads(hn) if isinstance(hn, str) else hn
        for lname, layer in hn_d.get("layers", {}).items():
            if layer.get("type") == "input_layer":
                input_layer_name = lname
                break
    except Exception as e:
        print(f"    warn: couldn't introspect HN ({e}); falling back to onnx name", flush=True)

    calib_key = input_layer_name or "hidden_states"
    print(f"    calibration dict key: {calib_key}", flush=True)
    # Hailo's HN treats inputs as 4D NCHW with implicit channels=1, so
    # [batch, seq, hidden] needs a channel dim → [batch, 1, seq, hidden].
    # Without this you get
    #   AccelerasValueError: Inference input shapes ... does not match HN shapes
    calib = {
        calib_key: rng.standard_normal((64, 1, SEQ_LEN, HIDDEN), dtype=np.float32),
    }
    runner.optimize(calib)
    opt_har = work / f"{NET_NAME}_optimized.har"
    runner.save_har(str(opt_har))
    print(f"    optimized HAR → {opt_har}", flush=True)

    print("==> [compile] hailo8 placement + scheduling (slow — minutes)", flush=True)
    hef = runner.compile()
    out_hef.write_bytes(hef)
    size = out_hef.stat().st_size
    print(f"    {size} bytes → {out_hef}", flush=True)


if __name__ == "__main__":
    if len(sys.argv) != 3:
        print(f"usage: {sys.argv[0]} <encoder_onnx> <out_hef>", file=sys.stderr)
        sys.exit(1)
    main(sys.argv[1], sys.argv[2])
