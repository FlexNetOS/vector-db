#!/usr/bin/env node

/**
 * Agentic Synth Examples CLI
 * Run production-ready examples directly
 */

import { Command } from 'commander';

const program = new Command();

program
  .name('agentic-synth-examples')
  .description('Production-ready examples for @ruvector/agentic-synth')
  .version('0.1.0')
  .addHelpText('after', `
Examples:
  $ agentic-synth-examples dspy train --models gemini,claude
  $ agentic-synth-examples self-learn --task code-generation
  $ agentic-synth-examples generate --type stock-market
  $ agentic-synth-examples list

Available Examples:
  dspy          - Multi-model DSPy training and benchmarking
  self-learn    - Self-learning and adaptive systems
  stock-market  - Financial market simulation
  cicd          - CI/CD pipeline test data
  security      - Security testing scenarios
  ad-roas       - Marketing campaign optimization
  swarm         - Multi-agent swarm coordination
  jujutsu       - Agentic-jujutsu version control

Learn more:
  https://www.npmjs.com/package/@ruvector/agentic-synth-examples
  https://github.com/FlexNetOS/ruvector/tree/main/packages/agentic-synth-examples
`);

program
  .command('list')
  .description('List all available examples')
  .action(() => {
    console.log(`
📚 Available Examples for @ruvector/agentic-synth

🧠 Machine Learning & AI:
  • dspy              - Multi-model DSPy training with optimization
  • self-learn        - Self-learning systems that improve over time
  • prompt-engineering - Automatic prompt optimization
  • model-benchmark   - Compare different AI models

💼 Business & Analytics:
  • ad-roas           - Marketing campaign optimization
  • employee-perf     - HR and workforce simulation
  • customer-analytics - User behavior and segmentation
  • revenue-forecast  - Financial prediction data

💰 Finance & Trading:
  • stock-market      - Realistic stock market data
  • crypto-trading    - Cryptocurrency market simulation
  • risk-analysis     - Financial risk scenarios
  • portfolio-opt     - Investment strategy data

🔒 Security & Testing:
  • security          - Penetration testing scenarios
  • log-analytics     - Security and monitoring logs
  • anomaly-detection - Unusual pattern generation
  • vulnerability     - Security test cases

🚀 DevOps & CI/CD:
  • cicd              - Pipeline testing data
  • deployment        - Release testing data
  • performance       - Load and stress test data
  • monitoring        - Alert and incident data

🤖 Agentic Systems:
  • swarm             - Multi-agent orchestration
  • agent-memory      - Context and memory patterns
  • jujutsu           - Version control for AI
  • distributed       - Federated learning examples

Usage:
  $ agentic-synth-examples <command> [options]
  $ agentic-synth-examples dspy train --models gemini
  $ agentic-synth-examples stock-market --count 1000

For more information:
  $ agentic-synth-examples <command> --help
`);
  });

program
  .command('dspy')
  .description('DSPy multi-model training and optimization')
  .argument('[subcommand]', 'train, benchmark, or optimize')
  .option('-m, --models <models>', 'Comma-separated model providers')
  .option('-r, --rounds <number>', 'Optimization rounds', '5')
  .option('-c, --convergence <number>', 'Quality threshold', '0.95')
  .option('-o, --output <path>', 'Output file path')
  .action((subcommand, options) => {
    console.log('🧠 DSPy Multi-Model Training\n');
    console.log('This example demonstrates training multiple AI models');
    console.log('with automatic prompt optimization using DSPy.ts.\n');
    console.log('Configuration:');
    console.log(`  Models: ${options.models || 'gemini,claude,gpt4'}`);
    console.log(`  Rounds: ${options.rounds}`);
    console.log(`  Convergence: ${options.convergence}`);
    console.log('\n⚠️  Note: Full implementation coming in v0.2.0');
    console.log('For now, see the source code in training/dspy-learning-session.ts');
  });

program
  .command('self-learn')
  .description('Self-learning adaptive generation systems')
  .option('-t, --task <task>', 'Task type (code-generation, text-summary, etc.)')
  .option('-i, --iterations <number>', 'Learning iterations', '10')
  .option('-l, --learning-rate <rate>', 'Learning rate', '0.1')
  .action((options) => {
    console.log('🔄 Self-Learning System\n');
    console.log('This example shows how to build systems that improve');
    console.log('their output quality automatically through feedback loops.\n');
    console.log('Configuration:');
    console.log(`  Task: ${options.task || 'general'}`);
    console.log(`  Iterations: ${options.iterations}`);
    console.log(`  Learning Rate: ${options.learningRate}`);
    console.log('\n⚠️  Note: Full implementation coming in v0.2.0');
  });

program
  .command('generate')
  .description('Generate example synthetic data')
  .option('-t, --type <type>', 'Data type (stock-market, cicd, security, etc.)')
  .option('-c, --count <number>', 'Number of records', '100')
  .option('-o, --output <path>', 'Output file path')
  .action((options) => {
    console.log(`📊 Generating ${options.type || 'generic'} data\n`);
    console.log(`Count: ${options.count} records`);
    if (options.output) {
      console.log(`Output: ${options.output}`);
    }
    console.log('\n⚠️  Note: Full implementation coming in v0.2.0');
    console.log('Use the main @ruvector/agentic-synth package for generation now.');
  });

// Error handler for unknown commands
program.on('command:*', function () {
  console.error('Invalid command: %s\nSee --help for a list of available commands.', program.args.join(' '));
  process.exit(1);
});

// Show help if no command provided
if (process.argv.length === 2) {
  program.help();
}

program.parse();
