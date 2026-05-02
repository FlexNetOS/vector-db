#!/usr/bin/env python3
"""Export only the BERT encoder layers of all-MiniLM-L6-v2 to ONNX.

Iter 139 follow-up to ADR-167's HEF model surgery scope. Skips:
  * `word_embeddings.Gather` (host-side embedding lookup will replace)
  * `Where`/`Expand` attention mask broadcast (host pre-computes the
    additive bias and passes it as a fully-expanded 4D tensor)

Inputs:
  hidden_states          [batch, seq, 384]   float32  — host pre-computed embeddings
  extended_attention_mask [batch, 1, 1, seq]  float32 — host pre-computed mask (0 or -10000)

Output:
  last_hidden_state      [batch, seq, 384]   float32

Iter 139: probe whether the Hailo Dataflow Compiler can fuse this
slimmed-down graph. If yes, the HEF model surgery in ADR-167 is unblocked
and we proceed to wire the host-side embedding lookup + mask construction
in HailoEmbedder. If no (Hailo still rejects the encoder's internal
ops), we know more about what surgery is actually required.
"""

import os
import sys
from pathlib import Path

os.environ.setdefault("TRANSFORMERS_NO_TF", "1")
os.environ.setdefault("USE_TF", "0")
os.environ.setdefault("TRANSFORMERS_NO_FLAX", "1")

import torch
from transformers import AutoModel

MODEL_NAME = "sentence-transformers/all-MiniLM-L6-v2"
OPSET = 14
SEQ_LEN = 128
HIDDEN = 384


class EncoderOnly(torch.nn.Module):
    """Wraps BertEncoder so it takes only hidden_states as input.

    The attention mask is baked in as a constant zero (no padding —
    full sequence attended). Trade-off: the worker must always pad to
    SEQ_LEN tokens; partial sequences get the same shape but with the
    right tokens. Works fine for sentence embeddings since shorter
    inputs are padded with [PAD] tokens anyway, and post-NPU mean-pool
    can apply the real mask host-side over the encoder output.

    Single-input form sidesteps the SDK's multi-input LayerNorm
    decomposition KeyError (iter 139 first attempt)."""

    def __init__(self, model):
        super().__init__()
        self.encoder = model.encoder

    def forward(self, hidden_states):
        out = self.encoder(
            hidden_states=hidden_states,
            attention_mask=None,  # full attention; host masks the output
            return_dict=True,
        )
        return out.last_hidden_state


def main(out_dir: str) -> None:
    out = Path(out_dir)
    out.mkdir(parents=True, exist_ok=True)
    onnx_path = out / "encoder.onnx"

    print(f"==> loading {MODEL_NAME}", flush=True)
    model = AutoModel.from_pretrained(MODEL_NAME).eval()
    encoder_only = EncoderOnly(model).eval()

    print(f"==> dummy inputs (batch=1, seq={SEQ_LEN}, hidden={HIDDEN})", flush=True)
    hidden_states = torch.randn(1, SEQ_LEN, HIDDEN)

    print(f"==> torch.onnx.export → {onnx_path}", flush=True)
    torch.onnx.export(
        encoder_only,
        (hidden_states,),
        str(onnx_path),
        input_names=["hidden_states"],
        output_names=["last_hidden_state"],
        opset_version=OPSET,
        do_constant_folding=True,
        # Fixed batch=1 — Hailo HEFs are compiled with concrete shapes
        # anyway, so dynamic batching gains us nothing on the export side.
    )

    size = onnx_path.stat().st_size
    print(f"    {size} bytes → {onnx_path}", flush=True)


if __name__ == "__main__":
    if len(sys.argv) != 2:
        print(f"usage: {sys.argv[0]} <output_dir>", file=sys.stderr)
        sys.exit(1)
    main(sys.argv[1])
