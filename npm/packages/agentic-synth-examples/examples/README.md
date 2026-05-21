# Agentic-Synth Examples - Progressive Tutorials

Complete, runnable tutorials for learning **agentic-synth** and **DSPy.ts** integration from beginner to advanced.

## 📚 Tutorial Structure

### 🟢 Beginner Level
Perfect for getting started with synthetic data generation and DSPy training.

### 🟡 Intermediate Level
Learn multi-model comparison, self-learning systems, and optimization.

### 🔴 Advanced Level
Build production-grade systems with custom learning and complete pipelines.

---

## 🚀 Quick Start

### Prerequisites

```bash
# Install dependencies
npm install dspy.ts @ruvector/agentic-synth

# Set up API keys
export GEMINI_API_KEY="your-gemini-api-key"
export ANTHROPIC_API_KEY="your-anthropic-key"  # Optional, for multi-model
export OPENAI_API_KEY="your-openai-key"        # Optional, for multi-model
```

### Running Tutorials

```bash
# From the package root
npx tsx examples/beginner/first-dspy-training.ts
npx tsx examples/intermediate/multi-model-comparison.ts
npx tsx examples/advanced/production-pipeline.ts
```

---

## 📖 Tutorial Catalog

### 🟢 Beginner Tutorials

#### 1. First DSPy Training (`beginner/first-dspy-training.ts`)

**Learn:** Basic DSPy.ts training with a single model

**Concepts:**
- Setting up DSPy language models
- Defining signatures for tasks
- Chain-of-Thought reasoning
- Simple evaluation metrics
- Training with examples

**Run:**
```bash
npx tsx examples/beginner/first-dspy-training.ts
```

**Output:**
```
🚀 Starting Your First DSPy Training Session

📊 Training with 3 examples...
✅ Training complete!

🧪 Testing the model with new products:

📦 Product: Smart Watch Pro
   Quality Score: 85%
   ✅ Excellent
```

**What You'll Build:** A product description generator that learns from examples

---

#### 2. Simple Data Generation (`beginner/simple-data-generation.ts`)

**Learn:** Generate structured synthetic data with schemas

**Concepts:**
- Defining data schemas
- Structured data generation
- Working with different formats (JSON, CSV)
- Saving output to files
- Using constraints for realistic data

**Run:**
```bash
npx tsx examples/beginner/simple-data-generation.ts
```

**Output:**
```
🎯 Simple Data Generation Tutorial

📊 Generating 5 sample users...

✅ Generation Complete!
Generated 5 users in 1234ms

👥 Generated Users:

1. John Smith (admin)
   📧 john.smith@example.com
   🎂 Age: 34
   🏠 San Francisco, USA

💾 Data saved to: examples/output/sample-users.json
```

**What You'll Build:** A user data generator for testing and prototyping

---

### 🟡 Intermediate Tutorials

#### 3. Multi-Model Comparison (`intermediate/multi-model-comparison.ts`)

**Learn:** Compare multiple AI models to find the best performer

**Concepts:**
- Running parallel model benchmarks
- Quality scoring across models
- Performance and speed metrics
- Cost tracking and optimization
- Selecting models for production

**Run:**
```bash
npx tsx examples/intermediate/multi-model-comparison.ts
```

**Output:**
```
🏆 Multi-Model Comparison Benchmark

📊 BENCHMARK RESULTS

┌─────────────────────┬──────────┬──────────┬──────────┬──────────┐
│ Model               │ Quality  │ Speed    │ Cost     │ Success  │
├─────────────────────┼──────────┼──────────┼──────────┼──────────┤
│ 🥇 GPT-4 Turbo      │   94.5%  │   892ms  │ $0.0023  │   100%   │
│ 🥈 Gemini Flash     │   89.2%  │   423ms  │ $0.0004  │   100%   │
│ 🥉 Claude Sonnet 4  │   91.8%  │   654ms  │ $0.0012  │   100%   │
└─────────────────────┴──────────┴──────────┴──────────┴──────────┘

🎯 WINNER: GPT-4 Turbo

💡 RECOMMENDATIONS:
⚡ Fastest: Gemini Flash (423ms avg)
💰 Cheapest: Gemini Flash ($0.0004 total)
🎯 Most Reliable: All models (100% success)
```

**What You'll Build:** A comprehensive model benchmarking system

---

#### 4. Self-Learning System (`intermediate/self-learning-system.ts`)

**Learn:** Build AI systems that improve over time through feedback

**Concepts:**
- Feedback loops for quality improvement
- Adaptive prompt engineering
- Pattern recognition from successes
- Tracking improvement over iterations
- Learning from mistakes

**Run:**
```bash
npx tsx examples/intermediate/self-learning-system.ts
```

**Output:**
```
🧠 Starting Self-Learning Session

📊 Iteration 1/8
   Quality: 65.0%
   ⚠️  Weaknesses: Description too short

🔧 Adapting strategy:
   • Expand description with more details

📊 Iteration 5/8
   Quality: 85.0%
   ✅ Target quality reached!

🎓 LEARNING SUMMARY
Quality Progression:
   Iteration 1: ████████████████ 65.0%
   Iteration 2: ████████████████████ 72.0%
   Iteration 3: ██████████████████████ 78.0%
   Iteration 4: ████████████████████████ 82.0%
   Iteration 5: ██████████████████████████ 85.0%

Improvement: +20.0% (+30.8%)
```

**What You'll Build:** An adaptive generator that learns from feedback

---

### 🔴 Advanced Tutorials

#### 5. Custom Learning System (`advanced/custom-learning-system.ts`)

**Learn:** Extend self-learning with custom evaluation and domain-specific optimization

**Concepts:**
- Custom multi-objective evaluators
- Domain-specific learning strategies
- Progressive difficulty training
- Knowledge base management
- Transfer learning patterns
- Few-shot learning from examples

**Run:**
```bash
npx tsx examples/advanced/custom-learning-system.ts
```

**Output:**
```
🏋️  Starting Advanced Training Session

Domain: ecommerce
Strategy: adaptive

📚 Phase 1: Learning Basics (Easy Examples)
📚 Phase 2: Intermediate Concepts (Medium Examples)
📚 Phase 3: Advanced Patterns (Hard Examples)

🎓 TRAINING RESULTS

Knowledge Base: 8 high-quality examples
Average Quality: 87.3%

Learned Categories:
  • electronics: 4 examples
  • fitness: 2 examples
  • photography: 2 examples

🧪 Testing Trained System

Test 1/3: Wireless Earbuds
📊 Metrics:
   Overall: 89.2%
   Accuracy: 92% | Creativity: 88%
   Relevance: 90% | Engagement: 85%

📈 TEST SUMMARY
Overall Performance: 87.8%
```

**What You'll Build:** A sophisticated domain-specific learning system

---

#### 6. Production Pipeline (`advanced/production-pipeline.ts`)

**Learn:** Build production-ready data generation with monitoring and controls

**Concepts:**
- Error handling and retry logic
- Rate limiting and cost controls
- Batch processing with concurrency
- Quality validation
- Comprehensive metrics tracking
- Results persistence
- Performance monitoring

**Run:**
```bash
npx tsx examples/advanced/production-pipeline.ts
```

**Output:**
```
🏭 Starting Production Pipeline

Configuration:
  Total Requests: 25
  Batch Size: 5
  Max Concurrency: 2
  Cost Budget: $1.00
  Rate Limit: 30/min

📦 Processing 5 batches...

Batch 1/5 (5 items)
✓ Batch complete: 5/5 successful
  Cost so far: $0.0005
  Cache hits: 0

📊 PIPELINE METRICS

Performance:
  Total Time: 12.34s
  Avg Request Time: 456ms
  Throughput: 2.02 req/s

Reliability:
  Total Requests: 25
  Successful: 24 (96.0%)
  Failed: 1
  Retries: 2

Cost & Efficiency:
  Total Cost: $0.0024
  Avg Cost/Request: $0.000096
  Cache Hit Rate: 32.0%
  Cost Savings from Cache: $0.0008

💾 Results saved to: output/production/generation-2025-01-15T10-30-45.json
📊 Metrics saved to: output/production/metrics-2025-01-15T10-30-45.json
```

**What You'll Build:** An enterprise-grade data generation pipeline

---

## 🎯 Learning Path

### Recommended Order:

1. **Start Here:** `beginner/first-dspy-training.ts`
   - Get comfortable with DSPy basics
   - Understand training concepts

2. **Then:** `beginner/simple-data-generation.ts`
   - Learn agentic-synth API
   - Practice schema definition

3. **Next:** `intermediate/multi-model-comparison.ts`
   - Compare model performance
   - Understand cost/quality tradeoffs

4. **Continue:** `intermediate/self-learning-system.ts`
   - Build adaptive systems
   - Implement feedback loops

5. **Advanced:** `advanced/custom-learning-system.ts`
   - Create domain-specific systems
   - Multi-objective optimization

6. **Finally:** `advanced/production-pipeline.ts`
   - Production patterns
   - Monitoring and reliability

---

## 💡 Key Concepts

### DSPy Integration
All tutorials demonstrate DSPy.ts integration with agentic-synth:
- **Language Models:** Configure AI providers
- **Signatures:** Define input/output structures
- **Chain-of-Thought:** Step-by-step reasoning
- **Optimizers:** BootstrapFewShot, MIPROv2

### Quality Evaluation
Learn multiple evaluation approaches:
- **Basic Metrics:** Length, completeness
- **Advanced Metrics:** Creativity, relevance, engagement
- **Multi-Objective:** Balance multiple goals
- **Domain-Specific:** Custom validators

### Production Patterns
Essential patterns for real-world use:
- **Error Handling:** Retries, fallbacks, recovery
- **Rate Limiting:** API quota management
- **Cost Control:** Budget tracking, optimization
- **Monitoring:** Metrics, logging, alerting
- **Caching:** Performance optimization

---

## 🛠️ Customization

### Modify for Your Use Case

Each tutorial is designed to be customized:

```typescript
// Change the domain
const domain = 'healthcare';  // or 'finance', 'legal', etc.

// Adjust schemas
const schema = {
  // Your custom fields
};

// Custom evaluation
class CustomEvaluator {
  evaluate(output: any): number {
    // Your logic
  }
}

// Different models
const models = ['gemini', 'claude', 'gpt4', 'llama'];
```

---

## 📊 Expected Results

### Performance Benchmarks

| Tutorial | Runtime | API Calls | Est. Cost |
|----------|---------|-----------|-----------|
| First DSPy Training | 30-60s | 5-10 | $0.01 |
| Simple Data Generation | 10-30s | 2-5 | $0.005 |
| Multi-Model Comparison | 2-5min | 12-30 | $0.15 |
| Self-Learning System | 1-3min | 8-15 | $0.02 |
| Custom Learning | 3-6min | 15-30 | $0.05 |
| Production Pipeline | 1-2min | 20-50 | $0.10 |

*Costs are estimates and vary by model and usage*

---

## 🔧 Troubleshooting

### Common Issues

**API Key Not Set:**
```bash
# Error: API key not configured
export GEMINI_API_KEY="your-key-here"
```

**Module Not Found:**
```bash
# Run from package root
cd packages/agentic-synth-examples
npm install
```

**Rate Limit Errors:**
```typescript
// Adjust in pipeline config
rateLimitPerMinute: 10  // Lower the rate
```

**Cost Budget Exceeded:**
```typescript
// Increase budget or reduce requests
costBudget: 5.0  // Higher budget
```

---

## 📚 Additional Resources

### Documentation
- [Agentic-Synth Main Docs](../README.md)
- [DSPy.ts Documentation](https://github.com/XpressAI/dspy.ts)
- [API Reference](../docs/api.md)

### Related Examples
- [Production Use Cases](../examples/use-cases/)
- [Integration Patterns](../examples/integrations/)
- [Testing Strategies](../examples/testing/)

---

## 🤝 Contributing

Have an idea for a tutorial?

1. Create your example file
2. Add comprehensive comments
3. Include error handling
4. Test thoroughly
5. Submit a pull request

---

## 📞 Support

- **Issues:** [GitHub Issues](https://github.com/FlexNetOS/ruvector/issues)
- **Discussions:** [GitHub Discussions](https://github.com/FlexNetOS/ruvector/discussions)
- **Questions:** Tag us on Twitter [@ruvnet](https://twitter.com/ruvnet)

---

## 📄 License

MIT © [ruvnet](https://github.com/ruvnet)

---

**Ready to learn?** Start with the [First DSPy Training tutorial](beginner/first-dspy-training.ts)! 🚀
