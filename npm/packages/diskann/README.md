# @ruvector/diskann

DiskANN/Vamana approximate nearest neighbor search — built in Rust, runs on all platforms.

Implements the Vamana graph algorithm from ["DiskANN: Fast Accurate Billion-point Nearest Neighbor Search on a Single Node" (NeurIPS 2019)](https://proceedings.neurips.cc/paper/2019/hash/09853c7fb1d3f8ee67a61b6bf4a7f8e6-Abstract.html).

## Install

```bash
npm install @ruvector/diskann
```

## Usage

```javascript
const { DiskAnn } = require('@ruvector/diskann');

const index = new DiskAnn({ dim: 128 });

// Insert vectors
for (let i = 0; i < 1000; i++) {
  const vec = new Float32Array(128);
  for (let d = 0; d < 128; d++) vec[d] = Math.random();
  index.insert(`vec-${i}`, vec);
}

// Build Vamana graph
index.build();

// Search
const query = new Float32Array(128).fill(0.5);
const results = index.search(query, 10);
console.log(results); // [{ id: 'vec-42', distance: 0.123 }, ...]

// Persist
index.save('./my-index');
const loaded = DiskAnn.load('./my-index');
```

## Performance

| Metric | Value |
|--------|-------|
| Search latency | **55µs** (5K vectors, 128d, k=10) |
| Recall@10 | **0.998** |
| Build | ~6s for 5K vectors |

## API

See full documentation at [github.com/FlexNetOS/ruvector](https://github.com/FlexNetOS/ruvector).

## License

MIT
