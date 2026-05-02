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
    """Wraps BertEncoder taking hidden_states + softmax mask as inputs.

    Iter 144 — adopts Hailo Model Zoo's official BERT pattern (see
    cfg/networks/bert_base_uncased.yaml). They split the network at
    /embeddings/Add_1 (post-embedding hidden states) AND the mask
    broadcast intermediate, then use `set_input_mask_to_softmax()` in
    the alls script to tell the SDK how to fold the mask into each
    softmax. This bypasses the iter-139/142 SDK chain
    (Where → KeyError → ElementwiseAddDirectOp deserialize) by going
    through the SDK's well-tested transformer codepath.

    Inputs:
      hidden_states           [batch, seq, hidden]  float32 — host-computed embeddings
      attention_softmax_mask  [batch, 1, 1, seq]    float32 — additive bias 0/-10000

    The attention_softmax_mask is what gets added to the QK^T scores
    pre-softmax in standard self-attention. Host computes it from the
    [batch, seq] padding mask once, broadcasts to 4D, sends as input."""

    def __init__(self, model):
        super().__init__()
        self.encoder = model.encoder

    def forward(self, hidden_states, attention_softmax_mask):
        out = self.encoder(
            hidden_states=hidden_states,
            attention_mask=attention_softmax_mask,
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
    attention_softmax_mask = torch.zeros(1, 1, 1, SEQ_LEN)

    print(f"==> torch.onnx.export → {onnx_path}", flush=True)
    torch.onnx.export(
        encoder_only,
        (hidden_states, attention_softmax_mask),
        str(onnx_path),
        input_names=["hidden_states", "attention_softmax_mask"],
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
