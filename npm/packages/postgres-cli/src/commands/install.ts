/**
 * RuVector PostgreSQL Installation Commands
 *
 * Provides complete installation of RuVector PostgreSQL extension:
 * - Docker-based installation (recommended)
 * - Native installation with pre-built binaries
 * - Extension management (enable, disable, upgrade)
 */

import { execSync, spawn, exec } from 'child_process';
import { promisify } from 'util';
import * as fs from 'fs';
import * as path from 'path';
import * as os from 'os';
import * as https from 'https';
import chalk from 'chalk';
import ora from 'ora';

const execAsync = promisify(exec);

// Constants
const DOCKER_IMAGE = 'ruvector-postgres';  // Local image name
const DOCKER_IMAGE_VERSION = '0.2.3';
const GITHUB_RELEASES_URL = 'https://api.github.com/repos/ruvnet/ruvector/releases/latest';
const DEFAULT_PORT = 5432;
const DEFAULT_USER = 'ruvector';
const DEFAULT_PASSWORD = 'ruvector';
const DEFAULT_DB = 'ruvector';

interface InstallOptions {
  method?: 'docker' | 'native' | 'auto';
  port?: number;
  user?: string;
  password?: string;
  database?: string;
  dataDir?: string;
  version?: string;
  detach?: boolean;
  name?: string;
}

interface StatusInfo {
  installed: boolean;
  running: boolean;
  method: 'docker' | 'native' | 'none';
  version?: string;
  containerId?: string;
  port?: number;
  connectionString?: string;
}

export class InstallCommands {

  /**
   * Check system requirements
   */
  static async checkRequirements(): Promise<{ docker: boolean; postgres: boolean; pgConfig: string | null }> {
    const result = { docker: false, postgres: false, pgConfig: null as string | null };

    // Check Docker
    try {
      execSync('docker --version', { stdio: 'pipe' });
      result.docker = true;
    } catch {
      result.docker = false;
    }

    // Check PostgreSQL
    try {
      execSync('psql --version', { stdio: 'pipe' });
      result.postgres = true;
    } catch {
      result.postgres = false;
    }

    // Check pg_config
    try {
      result.pgConfig = execSync('pg_config --libdir', { stdio: 'pipe', encoding: 'utf-8' }).trim();
    } catch {
      result.pgConfig = null;
    }

    return result;
  }

  /**
   * Install RuVector PostgreSQL (auto-detect best method)
   */
  static async install(options: InstallOptions = {}): Promise<void> {
    const spinner = ora('Checking system requirements...').start();

    try {
      const reqs = await this.checkRequirements();
      spinner.succeed('System check complete');

      console.log(chalk.bold('\n📋 System Status:'));
      console.log(`  Docker: ${reqs.docker ? chalk.green('✓ Available') : chalk.yellow('✗ Not found')}`);
      console.log(`  PostgreSQL: ${reqs.postgres ? chalk.green('✓ Available') : chalk.yellow('✗ Not found')}`);

      const method = options.method || 'auto';

      if (method === 'auto') {
        if (reqs.docker) {
          console.log(chalk.cyan('\n→ Using Docker installation (recommended)\n'));
          await this.installDocker(options);
        } else if (reqs.postgres && reqs.pgConfig) {
          console.log(chalk.cyan('\n→ Using native installation\n'));
          await this.installNative(options);
        } else {
          throw new Error('Neither Docker nor PostgreSQL found. Please install Docker or PostgreSQL first.');
        }
      } else if (method === 'docker') {
        if (!reqs.docker) {
          throw new Error('Docker not found. Please install Docker first: https://docs.docker.com/get-docker/');
        }
        await this.installDocker(options);
      } else if (method === 'native') {
        if (!reqs.postgres) {
          throw new Error('PostgreSQL not found. Please install PostgreSQL first.');
        }
        await this.installNative(options);
      }
    } catch (error) {
      spinner.fail('Installation failed');
      throw error;
    }
  }

  /**
   * Install via Docker
   */
  static async installDocker(options: InstallOptions = {}): Promise<void> {
    const port = options.port || DEFAULT_PORT;
    const user = options.user || DEFAULT_USER;
    const password = options.password || DEFAULT_PASSWORD;
    const database = options.database || DEFAULT_DB;
    const version = options.version || DOCKER_IMAGE_VERSION;
    const containerName = options.name || 'ruvector-postgres';
    const dataDir = options.dataDir;

    // Check if container already exists
    const existingSpinner = ora('Checking for existing installation...').start();
    try {
      const existing = execSync(`docker ps -a --filter name=${containerName} --format "{{.ID}}"`, { encoding: 'utf-8' }).trim();
      if (existing) {
        existingSpinner.warn(`Container '${containerName}' already exists`);
        console.log(chalk.yellow(`  Run 'ruvector-pg uninstall' first or use a different --name`));
        return;
      }
      existingSpinner.succeed('No existing installation found');
    } catch {
      existingSpinner.succeed('No existing installation found');
    }

    // Check for local image first, then try to pull, then build
    const pullSpinner = ora(`Checking for ${DOCKER_IMAGE}:${version}...`).start();
    try {
      // Check if image exists locally
      execSync(`docker image inspect ${DOCKER_IMAGE}:${version}`, { stdio: 'pipe' });
      pullSpinner.succeed(`Found local image ${DOCKER_IMAGE}:${version}`);
    } catch {
      // Try pulling from Docker Hub
      pullSpinner.text = `Pulling ${DOCKER_IMAGE}:${version}...`;
      try {
        execSync(`docker pull ${DOCKER_IMAGE}:${version}`, { stdio: 'pipe' });
        pullSpinner.succeed(`Pulled ${DOCKER_IMAGE}:${version}`);
      } catch {
        // Try ruvector/postgres from Docker Hub
        pullSpinner.text = 'Trying ruvector/postgres from Docker Hub...';
        try {
          execSync(`docker pull ruvector/postgres:${version}`, { stdio: 'pipe' });
          execSync(`docker tag ruvector/postgres:${version} ${DOCKER_IMAGE}:${version}`, { stdio: 'pipe' });
          pullSpinner.succeed(`Pulled ruvector/postgres:${version}`);
        } catch {
          pullSpinner.fail('Image not found locally or on Docker Hub');
          console.log(chalk.yellow('\n📦 To build the image locally, run:'));
          console.log(chalk.gray('   docker build -f crates/ruvector-postgres/docker/Dockerfile -t ruvector-postgres:0.2.3 .'));
          console.log(chalk.yellow('\n   Then run this install command again.\n'));
          throw new Error(`RuVector Docker image not available. Build it first or check Docker Hub.`);
        }
      }
    }

    // Build run command
    let runCmd = `docker run -d --name ${containerName}`;
    runCmd += ` -p ${port}:5432`;
    runCmd += ` -e POSTGRES_USER=${user}`;
    runCmd += ` -e POSTGRES_PASSWORD=${password}`;
    runCmd += ` -e POSTGRES_DB=${database}`;

    if (dataDir) {
      const absDataDir = path.resolve(dataDir);
      if (!fs.existsSync(absDataDir)) {
        fs.mkdirSync(absDataDir, { recursive: true });
      }
      runCmd += ` -v ${absDataDir}:/var/lib/postgresql/data`;
    }

    runCmd += ` ${DOCKER_IMAGE}:${version}`;

    // Run container
    const runSpinner = ora('Starting RuVector PostgreSQL...').start();
    try {
      const containerId = execSync(runCmd, { encoding: 'utf-8' }).trim();
      runSpinner.succeed('Container started');

      // Wait for PostgreSQL to be ready
      const readySpinner = ora('Waiting for PostgreSQL to be ready...').start();
      let ready = false;
      for (let i = 0; i < 30; i++) {
        try {
          execSync(`docker exec ${containerName} pg_isready -U ${user}`, { stdio: 'pipe' });
          ready = true;
          break;
        } catch {
          await new Promise(resolve => setTimeout(resolve, 1000));
        }
      }

      if (ready) {
        readySpinner.succeed('PostgreSQL is ready');
      } else {
        readySpinner.warn('PostgreSQL may still be starting...');
      }

      // Verify extension
      const verifySpinner = ora('Verifying RuVector extension...').start();
      try {
        const extCheck = execSync(
          `docker exec ${containerName} psql -U ${user} -d ${database} -c "SELECT extname, extversion FROM pg_extension WHERE extname = 'ruvector';"`,
          { encoding: 'utf-8' }
        );
        if (extCheck.includes('ruvector')) {
          verifySpinner.succeed('RuVector extension verified');
        } else {
          verifySpinner.warn('Extension may need manual activation');
        }
      } catch {
        verifySpinner.warn('Could not verify extension (database may still be initializing)');
      }

      // Print success message
      console.log(chalk.green.bold('\n✅ RuVector PostgreSQL installed successfully!\n'));
      console.log(chalk.bold('Connection Details:'));
      console.log(`  Host:     ${chalk.cyan('localhost')}`);
      console.log(`  Port:     ${chalk.cyan(port.toString())}`);
      console.log(`  User:     ${chalk.cyan(user)}`);
      console.log(`  Password: ${chalk.cyan(password)}`);
      console.log(`  Database: ${chalk.cyan(database)}`);
      console.log(`  Container: ${chalk.cyan(containerName)}`);

      const connString = `postgresql://${user}:${password}@localhost:${port}/${database}`;
      console.log(chalk.bold('\nConnection String:'));
      console.log(`  ${chalk.cyan(connString)}`);

      console.log(chalk.bold('\nQuick Start:'));
      console.log(`  ${chalk.gray('# Connect with psql')}`);
      console.log(`  psql "${connString}"`);
      console.log(`  ${chalk.gray('# Or use docker')}`);
      console.log(`  docker exec -it ${containerName} psql -U ${user} -d ${database}`);

      console.log(chalk.bold('\nTest HNSW Index:'));
      console.log(chalk.gray(`  CREATE TABLE items (id serial, embedding real[]);`));
      console.log(chalk.gray(`  CREATE INDEX ON items USING hnsw (embedding);`));

    } catch (error) {
      runSpinner.fail('Failed to start container');
      throw error;
    }
  }

  /**
   * Install native extension (download pre-built binaries)
   */
  static async installNative(options: InstallOptions = {}): Promise<void> {
    const spinner = ora('Detecting system...').start();

    const platform = os.platform();
    const arch = os.arch();

    spinner.text = `Detected: ${platform}-${arch}`;

    // Determine binary name
    let binaryName: string;
    if (platform === 'linux' && arch === 'x64') {
      binaryName = 'ruvector-pg16-linux-x64.tar.gz';
    } else if (platform === 'darwin' && arch === 'arm64') {
      binaryName = 'ruvector-pg16-darwin-arm64.tar.gz';
    } else if (platform === 'darwin' && arch === 'x64') {
      binaryName = 'ruvector-pg16-darwin-x64.tar.gz';
    } else {
      spinner.fail(`Unsupported platform: ${platform}-${arch}`);
      console.log(chalk.yellow('\nPre-built binaries not available for your platform.'));
      console.log(chalk.yellow('Please use Docker installation or build from source:'));
      console.log(chalk.gray('  cargo install cargo-pgrx'));
      console.log(chalk.gray('  cargo pgrx install'));
      return;
    }

    spinner.succeed(`System: ${platform}-${arch}`);

    // Get pg_config paths
    const pgConfigSpinner = ora('Getting PostgreSQL paths...').start();
    let libDir: string;
    let shareDir: string;

    try {
      libDir = execSync('pg_config --pkglibdir', { encoding: 'utf-8' }).trim();
      shareDir = execSync('pg_config --sharedir', { encoding: 'utf-8' }).trim();
      pgConfigSpinner.succeed('PostgreSQL paths found');
      console.log(`  Library dir: ${chalk.cyan(libDir)}`);
      console.log(`  Share dir: ${chalk.cyan(shareDir)}`);
    } catch {
      pgConfigSpinner.fail('Could not find pg_config');
      throw new Error('PostgreSQL development files not found. Install postgresql-server-dev-XX package.');
    }

    // Download release
    const downloadSpinner = ora('Fetching latest release info...').start();

    try {
      // For now, provide manual instructions
      // In production, this would download from GitHub releases
      downloadSpinner.info('Native installation requires manual steps');

      console.log(chalk.bold('\n📦 Manual Installation Steps:\n'));
      console.log('1. Download the pre-built extension:');
      console.log(chalk.gray(`   https://github.com/ruvnet/ruvector/releases/latest`));
      console.log(`   Look for: ${chalk.cyan(binaryName)}`);

      console.log('\n2. Extract and copy files:');
      console.log(chalk.gray(`   tar -xzf ${binaryName}`));
      console.log(chalk.gray(`   sudo cp ruvector.so ${libDir}/`));
      console.log(chalk.gray(`   sudo cp ruvector.control ${shareDir}/extension/`));
      console.log(chalk.gray(`   sudo cp ruvector--*.sql ${shareDir}/extension/`));

      console.log('\n3. Enable the extension:');
      console.log(chalk.gray(`   psql -c "CREATE EXTENSION ruvector;"`));

      console.log(chalk.yellow('\n💡 Tip: Use Docker for easier installation:'));
      console.log(chalk.gray('   ruvector-pg install --method docker'));

    } catch (error) {
      downloadSpinner.fail('Failed to get release info');
      throw error;
    }
  }

  /**
   * Uninstall RuVector PostgreSQL
   */
  static async uninstall(options: { name?: string; removeData?: boolean } = {}): Promise<void> {
    const containerName = options.name || 'ruvector-postgres';

    const spinner = ora(`Stopping container '${containerName}'...`).start();

    try {
      // Stop container
      try {
        execSync(`docker stop ${containerName}`, { stdio: 'pipe' });
        spinner.succeed('Container stopped');
      } catch {
        spinner.info('Container was not running');
      }

      // Remove container
      const removeSpinner = ora('Removing container...').start();
      try {
        execSync(`docker rm ${containerName}`, { stdio: 'pipe' });
        removeSpinner.succeed('Container removed');
      } catch {
        removeSpinner.info('Container already removed');
      }

      if (options.removeData) {
        console.log(chalk.yellow('\n⚠️  Data volumes were not removed (manual cleanup required)'));
      }

      console.log(chalk.green.bold('\n✅ RuVector PostgreSQL uninstalled\n'));

    } catch (error) {
      spinner.fail('Uninstall failed');
      throw error;
    }
  }

  /**
   * Get installation status
   */
  static async status(options: { name?: string } = {}): Promise<StatusInfo> {
    const containerName = options.name || 'ruvector-postgres';

    const info: StatusInfo = {
      installed: false,
      running: false,
      method: 'none',
    };

    // Check Docker installation
    try {
      const containerInfo = execSync(
        `docker inspect ${containerName} --format '{{.State.Running}} {{.Config.Image}} {{.NetworkSettings.Ports}}'`,
        { encoding: 'utf-8', stdio: ['pipe', 'pipe', 'pipe'] }
      ).trim();

      const [running, image] = containerInfo.split(' ');
      info.installed = true;
      info.running = running === 'true';
      info.method = 'docker';
      info.version = image.split(':')[1] || 'latest';
      info.containerId = execSync(`docker inspect ${containerName} --format '{{.Id}}'`, { encoding: 'utf-8' }).trim().substring(0, 12);

      // Get port mapping
      const portMapping = execSync(
        `docker port ${containerName} 5432`,
        { encoding: 'utf-8', stdio: ['pipe', 'pipe', 'pipe'] }
      ).trim();
      const portMatch = portMapping.match(/:(\d+)$/);
      if (portMatch) {
        info.port = parseInt(portMatch[1]);
        info.connectionString = `postgresql://ruvector:ruvector@localhost:${info.port}/ruvector`;
      }

    } catch {
      // No Docker installation found
    }

    return info;
  }

  /**
   * Print status information
   */
  static async printStatus(options: { name?: string } = {}): Promise<void> {
    const spinner = ora('Checking installation status...').start();

    const status = await this.status(options);
    spinner.stop();

    console.log(chalk.bold('\n📊 RuVector PostgreSQL Status\n'));

    if (!status.installed) {
      console.log(`  Status: ${chalk.yellow('Not installed')}`);
      console.log(chalk.gray('\n  Run `ruvector-pg install` to install'));
      return;
    }

    console.log(`  Installed: ${chalk.green('Yes')}`);
    console.log(`  Method: ${chalk.cyan(status.method)}`);
    console.log(`  Version: ${chalk.cyan(status.version || 'unknown')}`);
    console.log(`  Running: ${status.running ? chalk.green('Yes') : chalk.red('No')}`);

    if (status.method === 'docker') {
      console.log(`  Container: ${chalk.cyan(status.containerId)}`);
    }

    if (status.port) {
      console.log(`  Port: ${chalk.cyan(status.port.toString())}`);
    }

    if (status.connectionString) {
      console.log(`\n  Connection: ${chalk.cyan(status.connectionString)}`);
    }

    if (!status.running) {
      console.log(chalk.gray('\n  Run `ruvector-pg start` to start the database'));
    }
  }

  /**
   * Start the database
   */
  static async start(options: { name?: string } = {}): Promise<void> {
    const containerName = options.name || 'ruvector-postgres';
    const spinner = ora('Starting RuVector PostgreSQL...').start();

    try {
      execSync(`docker start ${containerName}`, { stdio: 'pipe' });

      // Wait for ready
      for (let i = 0; i < 30; i++) {
        try {
          execSync(`docker exec ${containerName} pg_isready`, { stdio: 'pipe' });
          spinner.succeed('RuVector PostgreSQL started');
          return;
        } catch {
          await new Promise(resolve => setTimeout(resolve, 1000));
        }
      }

      spinner.warn('Started but may not be ready yet');
    } catch (error) {
      spinner.fail('Failed to start');
      throw error;
    }
  }

  /**
   * Stop the database
   */
  static async stop(options: { name?: string } = {}): Promise<void> {
    const containerName = options.name || 'ruvector-postgres';
    const spinner = ora('Stopping RuVector PostgreSQL...').start();

    try {
      execSync(`docker stop ${containerName}`, { stdio: 'pipe' });
      spinner.succeed('RuVector PostgreSQL stopped');
    } catch (error) {
      spinner.fail('Failed to stop');
      throw error;
    }
  }

  /**
   * Show logs
   */
  static async logs(options: { name?: string; follow?: boolean; tail?: number } = {}): Promise<void> {
    const containerName = options.name || 'ruvector-postgres';
    const tail = options.tail || 100;

    let cmd = `docker logs ${containerName} --tail ${tail}`;
    if (options.follow) {
      cmd += ' -f';
    }

    try {
      if (options.follow) {
        const child = spawn('docker', ['logs', containerName, '--tail', tail.toString(), '-f'], {
          stdio: 'inherit'
        });
        child.on('error', (err) => {
          console.error(chalk.red(`Error: ${err.message}`));
        });
      } else {
        const output = execSync(cmd, { encoding: 'utf-8' });
        console.log(output);
      }
    } catch (error) {
      console.error(chalk.red('Failed to get logs'));
      throw error;
    }
  }

  /**
   * Execute psql command
   */
  static async psql(options: { name?: string; command?: string } = {}): Promise<void> {
    const containerName = options.name || 'ruvector-postgres';

    if (options.command) {
      try {
        const output = execSync(
          `docker exec ${containerName} psql -U ruvector -d ruvector -c "${options.command}"`,
          { encoding: 'utf-8' }
        );
        console.log(output);
      } catch (error) {
        console.error(chalk.red('Failed to execute command'));
        throw error;
      }
    } else {
      // Interactive mode
      const child = spawn('docker', ['exec', '-it', containerName, 'psql', '-U', 'ruvector', '-d', 'ruvector'], {
        stdio: 'inherit'
      });
      child.on('error', (err) => {
        console.error(chalk.red(`Error: ${err.message}`));
      });
    }
  }
}
