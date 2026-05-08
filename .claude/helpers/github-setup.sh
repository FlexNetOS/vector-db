#!/usr/bin/env bash
# Setup GitHub integration for Claude Flow
set -euo pipefail

echo "🔗 Setting up GitHub integration..."

# Install gh when possible. On Ubuntu/Debian runners and Devin VMs this is
# available from apt; on other platforms print the official install link.
if ! command -v gh >/dev/null 2>&1; then
    if command -v apt-get >/dev/null 2>&1; then
        echo "📦 Installing GitHub CLI (gh) via apt"
        sudo apt-get update
        sudo apt-get install -y gh || {
            echo "📦 Falling back to cli.github.com apt repository"
            sudo mkdir -p /etc/apt/keyrings
            curl -fsSL https://cli.github.com/packages/githubcli-archive-keyring.gpg | \
                sudo dd of=/etc/apt/keyrings/githubcli-archive-keyring.gpg >/dev/null
            sudo chmod go+r /etc/apt/keyrings/githubcli-archive-keyring.gpg
            echo "deb [arch=$(dpkg --print-architecture) signed-by=/etc/apt/keyrings/githubcli-archive-keyring.gpg] https://cli.github.com/packages stable main" | \
                sudo tee /etc/apt/sources.list.d/github-cli.list >/dev/null
            sudo apt-get update
            sudo apt-get install -y gh
        }
    else
        echo "⚠️  GitHub CLI (gh) not found"
        echo "Install from: https://cli.github.com/"
        echo "Continuing without GitHub features..."
    fi
fi

if command -v gh >/dev/null 2>&1; then
    echo "✅ GitHub CLI found: $(gh --version | head -n1)"

    # Check auth status
    if gh auth status >/dev/null 2>&1; then
        echo "✅ GitHub authentication active"
    else
        echo "⚠️  Not authenticated with GitHub"
        echo "Run: gh auth login"
    fi
fi

# Configure the canonical fork/upstream remotes for PR workflows.
git remote add origin https://github.com/FlexNetOS/ruvector.git 2>/dev/null || \
    git remote set-url origin https://github.com/FlexNetOS/ruvector.git
git remote add upstream https://github.com/ruvnet/RuVector.git 2>/dev/null || \
    git remote set-url upstream https://github.com/ruvnet/RuVector.git
echo "✅ Git remotes configured"
git remote -v

echo ""
echo "📦 GitHub swarm commands available:"
echo "  - npx claude-flow github swarm"
echo "  - npx claude-flow repo analyze"
echo "  - npx claude-flow pr enhance"
echo "  - npx claude-flow issue triage"
