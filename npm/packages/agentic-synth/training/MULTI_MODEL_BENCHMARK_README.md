# DSPy Multi-Model Benchmark Suite

Comprehensive benchmarking system for comparing multiple language models using real **dspy.ts v2.1.1** features.

## Features

### Real DSPy.ts Components

- ✅ **ChainOfThought** - For reasoning-based synthetic data generation
- ✅ **ReAct** - For iterative data quality validation
- ✅ **BootstrapFewShot** - Learn from successful examples (5 rounds)
- ✅ **MIPROv2** - Bayesian prompt optimization (3 trials)
- ✅ **Real Metrics** - f1Score, exactMatch, bleuScore, rougeScore

### Benchmark Capabilities

1. **Multi-Model Comparison**
   - OpenAI models (GPT-4, GPT-3.5-turbo)
   - Anthropic models (Claude 3 Sonnet, Claude 3 Haiku)
   - Automatic model registration and configuration

2. **Quality Metrics**
   - F1 Score
   - Exact Match
   - BLEU Score
   - ROUGE Score
   - Overall quality score

3. **Performance Metrics**
   - Latency (P50, P95, P99)
   - Throughput (samples/second)
   - Success rate
   - Average latency

4. **Cost Analysis**
   - Total cost tracking
   - Cost per sample
   - Cost per quality point
   - Token usage (input/output)

5. **Optimization Comparison**
   - Baseline quality
   - BootstrapFewShot improvement
   - MIPROv2 improvement
   - Quality progression tracking

## Installation

```bash
cd /home/user/ruvector/packages/agentic-synth
npm install
```

## Setup

Set your API keys as environment variables:

```bash
export OPENAI_API_KEY="your-openai-key"
export ANTHROPIC_API_KEY="your-anthropic-key"
```

Or create a `.env` file:

```env
OPENAI_API_KEY=your-openai-key
ANTHROPIC_API_KEY=your-anthropic-key
SAMPLE_SIZE=100
```

## Usage

### Basic Usage

```bash
npx tsx training/dspy-multi-model-benchmark.ts
```

### Custom Sample Size

```bash
SAMPLE_SIZE=1000 npx tsx training/dspy-multi-model-benchmark.ts
```

### Programmatic Usage

```typescript
import { DSPyMultiModelBenchmark } from './training/dspy-multi-model-benchmark';

const benchmark = new DSPyMultiModelBenchmark('./results');

// Add models
benchmark.addModel({
  name: 'GPT-4',
  provider: 'openai',
  modelId: 'gpt-4',
  apiKey: process.env.OPENAI_API_KEY,
  costPer1kTokens: { input: 0.03, output: 0.06 },
  maxTokens: 8192
});

// Run comparison
const results = await benchmark.runComparison(1000);

// Generate report
await benchmark.generateReport(results);
```

## Output

The benchmark generates two files:

1. **Markdown Report** (`benchmark-report-TIMESTAMP.md`)
   - Executive summary with winners
   - Detailed metrics for each model
   - Rankings by category
   - Recommendations for different use cases

2. **JSON Results** (`benchmark-results-TIMESTAMP.json`)
   - Complete benchmark data
   - Raw metrics
   - Optimization history
   - Structured for further analysis

### Sample Output Structure

```
training/results/multi-model/
├── benchmark-report-2025-01-22T10-30-45-123Z.md
└── benchmark-results-2025-01-22T10-30-45-123Z.json
```

## Benchmark Workflow

```
┌─────────────────────────────────────────────────────────┐
│                   For Each Model                        │
└─────────────────────────────────────────────────────────┘
                           │
                           ▼
┌─────────────────────────────────────────────────────────┐
│ 1. Baseline Quality                                     │
│    └─ Test with basic ChainOfThought module            │
└─────────────────────────────────────────────────────────┘
                           │
                           ▼
┌─────────────────────────────────────────────────────────┐
│ 2. BootstrapFewShot Optimization                        │
│    └─ 5 rounds of few-shot learning                    │
│    └─ Learn from successful examples                   │
└─────────────────────────────────────────────────────────┘
                           │
                           ▼
┌─────────────────────────────────────────────────────────┐
│ 3. MIPROv2 Optimization                                 │
│    └─ 3 trials of Bayesian optimization                │
│    └─ Expected Improvement acquisition                 │
└─────────────────────────────────────────────────────────┘
                           │
                           ▼
┌─────────────────────────────────────────────────────────┐
│ 4. Performance Testing                                  │
│    └─ Measure latency (P50, P95, P99)                  │
│    └─ Calculate throughput                             │
└─────────────────────────────────────────────────────────┘
                           │
                           ▼
┌─────────────────────────────────────────────────────────┐
│ 5. Cost Analysis                                        │
│    └─ Track token usage                                │
│    └─ Calculate cost efficiency                        │
└─────────────────────────────────────────────────────────┘
```

## Metrics Explained

### Quality Metrics

- **F1 Score**: Harmonic mean of precision and recall
- **Exact Match**: Percentage of exact matches with expected output
- **BLEU Score**: Bilingual Evaluation Understudy (text similarity)
- **ROUGE Score**: Recall-Oriented Understudy for Gisting Evaluation
- **Overall**: Weighted average of all quality metrics

### Performance Metrics

- **P50 Latency**: Median response time
- **P95 Latency**: 95th percentile response time
- **P99 Latency**: 99th percentile response time
- **Throughput**: Samples processed per second
- **Success Rate**: Percentage of successful generations

### Optimization Metrics

- **Baseline Quality**: Initial quality without optimization
- **Bootstrap Improvement**: Quality gain from BootstrapFewShot
- **MIPRO Improvement**: Quality gain from MIPROv2
- **Improvement %**: Relative improvement over baseline

## Customization

### Add Custom Models

```typescript
benchmark.addModel({
  name: 'Custom Model',
  provider: 'openrouter',
  modelId: 'model-id',
  apiKey: 'your-key',
  costPer1kTokens: { input: 0.001, output: 0.002 },
  maxTokens: 4096
});
```

### Custom Schema

Modify the schema in `benchmarkModel()`:

```typescript
const schema = {
  id: 'UUID',
  name: 'string (person name)',
  email: 'string (valid email)',
  age: 'number (18-80)',
  // Add your custom fields...
};
```

### Custom Metrics

Implement custom quality scoring:

```typescript
private calculateQualityScore(output: any, expected: any): number {
  // Your custom scoring logic
  return score;
}
```

## Performance Tips

1. **Start Small**: Use `SAMPLE_SIZE=10` for quick tests
2. **Increase Gradually**: Scale to 100, 1000, 10000 as needed
3. **Parallel Testing**: Run different models separately
4. **Cost Monitoring**: Check costs before large runs
5. **Rate Limits**: Be aware of API rate limits

## Example Results

```
🔬 DSPy Multi-Model Benchmark Suite
======================================================================
Models: 4
Sample Size: 100
======================================================================

📊 Benchmarking: GPT-4
----------------------------------------------------------------------
  → Running baseline...
  → Optimizing with BootstrapFewShot...
  → Optimizing with MIPROv2...
  ✓ Quality Score: 0.875
  ✓ P95 Latency: 1234ms
  ✓ Cost/Sample: $0.000543
  ✓ Bootstrap Improvement: +12.3%
  ✓ MIPRO Improvement: +18.7%

📊 Benchmarking: Claude 3 Sonnet
----------------------------------------------------------------------
  → Running baseline...
  → Optimizing with BootstrapFewShot...
  → Optimizing with MIPROv2...
  ✓ Quality Score: 0.892
  ✓ P95 Latency: 987ms
  ✓ Cost/Sample: $0.000234
  ✓ Bootstrap Improvement: +14.2%
  ✓ MIPRO Improvement: +21.5%

======================================================================
✅ Benchmark completed successfully!
📊 Check the results directory for detailed reports.
======================================================================
```

## Troubleshooting

### API Key Issues

```bash
# Check if keys are set
echo $OPENAI_API_KEY
echo $ANTHROPIC_API_KEY

# Set keys temporarily
export OPENAI_API_KEY="sk-..."
export ANTHROPIC_API_KEY="sk-ant-..."
```

### Import Errors

```bash
# Rebuild the package
npm run build

# Check dspy.ts installation
npm list dspy.ts
```

### Out of Memory

```bash
# Reduce sample size
SAMPLE_SIZE=10 npx tsx training/dspy-multi-model-benchmark.ts
```

### Rate Limiting

Add delays between requests:

```typescript
// In measurePerformance()
await new Promise(resolve => setTimeout(resolve, 100));
```

## Architecture

```
DSPyMultiModelBenchmark
├── Model Management
│   ├── OpenAILM (GPT-4, GPT-3.5)
│   ├── AnthropicLM (Claude 3)
│   └── Token tracking
│
├── DSPy Modules
│   ├── SyntheticDataModule (ChainOfThought)
│   └── DataQualityModule (ReAct)
│
├── Optimizers
│   ├── BootstrapFewShot (5 rounds)
│   └── MIPROv2 (3 trials, Bayesian)
│
├── Metrics
│   ├── Quality (F1, EM, BLEU, ROUGE)
│   ├── Performance (latency, throughput)
│   └── Cost (tokens, efficiency)
│
└── Reporting
    ├── Markdown reports
    └── JSON results
```

## Contributing

To add new features:

1. Extend `ModelConfig` for new providers
2. Implement new LM classes
3. Add custom DSPy modules
4. Enhance quality metrics
5. Extend reporting formats

## License

MIT - Same as dspy.ts and agentic-synth

## References

- [dspy.ts Documentation](https://github.com/FlexNetOS/dspy.ts)
- [DSPy Paper](https://arxiv.org/abs/2310.03714)
- [MIPROv2 Paper](https://arxiv.org/abs/2406.11695)

---

**Built with dspy.ts v2.1.1** - Declarative AI framework for TypeScript
