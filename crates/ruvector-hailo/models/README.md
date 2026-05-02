# ruvector-hailo / models — building the HEF

These binaries are not committed into the repository. To rebuild from the
upstream `sentence-transformers/all-MiniLM-L6-v2` ONNX:

## Prereqs (x86 Linux only — runs on ruvultra)

1. **Hailo Dataflow Compiler 3.x** (developer license required).
   Download from `https://hailo.ai/developer-zone/`.
2. `python3 -m venv ~/hailo-dfc-venv && source ~/hailo-dfc-venv/bin/activate`
3. `pip install hailo-dataflow-compiler-<version>-py3-none-linux_x86_64.whl`
4. `pip install onnx transformers sentence-transformers`

## Build steps

```bash
mkdir -p all-minilm-l6-v2 && cd all-minilm-l6-v2

# 1. Pull ONNX + tokenizer
python -c "
from sentence_transformers import SentenceTransformer
m = SentenceTransformer('sentence-transformers/all-MiniLM-L6-v2')
m.save('./')
import torch
torch.onnx.export(m._first_module().auto_model,
    (torch.zeros(1,128, dtype=torch.long),
     torch.ones(1,128, dtype=torch.long)),
    'model.onnx', input_names=['input_ids','attention_mask'],
    output_names=['last_hidden_state'], opset_version=14,
    dynamic_axes={'input_ids':{0:'batch'},'attention_mask':{0:'batch'},
                  'last_hidden_state':{0:'batch'}})
"

# 2. Calibration corpus (10k lines of plain English)
head -10000 ../../../bench_data/glove.6B.100d.txt | awk '{print $1}' > calib.txt

# 3. Hailo DFC parse → optimize → compile
hailo parser --hw-arch hailo8 --har-name all_minilm_l6_v2 model.onnx \
   --start-node-names input_ids attention_mask \
   --end-node-names last_hidden_state \
   --net-input-shapes input_ids=[1,128],attention_mask=[1,128]

hailo optimize --hw-arch hailo8 \
   --calib-set-path calib.txt \
   --use-random-calib-set \
   all_minilm_l6_v2.har

hailo compiler --hw-arch hailo8 all_minilm_l6_v2_optimized.har
mv all_minilm_l6_v2_optimized.hef model.hef

# 4. Sanity-check on Pi
scp model.hef vocab.txt special_tokens.json genesis@cognitum-v0:~/ruvector/crates/ruvector-hailo/models/all-minilm-l6-v2/
ssh genesis@cognitum-v0 hailortcli parse-hef ~/ruvector/crates/ruvector-hailo/models/all-minilm-l6-v2/model.hef
```

## Expected I/O shapes after compile

```
input  input_ids        [1, 128]  int32
input  attention_mask   [1, 128]  int32
output last_hidden_state [1, 128, 384] float32
```

Pooling (mean over sequence dim, masked by attention) is done on CPU after
the NPU emits the per-token embeddings — see `src/inference.rs` once
iteration 7 lands.
