/**
 * Worker Thread Pool Manager
 * Enables non-blocking parallel PDF processing
 */

import { Worker } from 'worker_threads';
import os from 'os';
import path from 'path';
import { fileURLToPath } from 'url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));

/**
 * Represents a task to be processed by a worker
 */
export interface WorkerTask<T = any> {
  operation: 'extract' | 'search' | 'render' | 'analyze';
  documentPath: string;
  params: Record<string, any>;
}

/**
 * Result returned from a worker
 */
export interface WorkerResult<T = any> {
  success: boolean;
  data?: T;
  error?: Error | string;
  duration: number;
}

interface QueuedTask {
  task: WorkerTask<any>;
  resolve: (value: WorkerResult<any>) => void;
  reject: (error: Error) => void;
  timeout: NodeJS.Timeout;
}

/**
 * Thread pool for parallel PDF processing
 */
export class WorkerPool {
  private workers: Worker[] = [];
  private queue: QueuedTask[] = [];
  private activeCount = 0;
  private terminated = false;
  private readonly defaultTimeout = 30000; // 30 seconds

  /**
   * Initialize the worker pool
   * @param poolSize - Number of worker threads to create
   */
  constructor(private poolSize: number = 4) {
    this.validatePoolSize();
    this.initializeWorkers();
  }

  private validatePoolSize(): void {
    if (this.poolSize < 1 || this.poolSize > 32) {
      throw new Error(
        `Pool size must be between 1 and 32, got ${this.poolSize}`
      );
    }
  }

  private initializeWorkers(): void {
    try {
      for (let i = 0; i < this.poolSize; i++) {
        const worker = new Worker(path.join(__dirname, 'worker.js'));

        worker.on('error', (error) => {
          console.error(`Worker ${i} error:`, error);
          this.handleWorkerError(error);
        });

        worker.on('exit', (code) => {
          if (code !== 0 && !this.terminated) {
            console.warn(`Worker ${i} exited with code ${code}`);
          }
        });

        this.workers.push(worker);
      }
    } catch (error) {
      this.cleanup();
      throw new Error(
        `Failed to initialize worker pool: ${
          error instanceof Error ? error.message : String(error)
        }`
      );
    }
  }

  /**
   * Run a task in the worker pool
   * @param task - The task to run
   * @param timeout - Optional timeout in milliseconds
   * @returns Promise that resolves with the result
   */
  public async runTask<T = any>(
    task: WorkerTask<T>,
    timeout: number = this.defaultTimeout
  ): Promise<WorkerResult<T>> {
    if (this.terminated) {
      throw new Error('Worker pool has been terminated');
    }

    if (timeout < 1000 || timeout > 300000) {
      throw new Error('Timeout must be between 1 and 300 seconds');
    }

    return new Promise<WorkerResult<T>>((resolve, reject) => {
      const timeoutHandle = setTimeout(() => {
        this.queue = this.queue.filter((q) => q.task !== task);
        reject(
          new Error(
            `Worker task timeout after ${timeout}ms: ${task.operation} on ${task.documentPath}`
          )
        );
      }, timeout);

      this.queue.push({
        task,
        resolve,
        reject,
        timeout: timeoutHandle,
      });

      this.processQueue();
    });
  }

  private processQueue(): void {
    if (this.queue.length === 0 || this.activeCount >= this.poolSize) {
      return;
    }

    const queuedTask = this.queue.shift();
    if (!queuedTask) return;

    const { task, resolve, reject, timeout } = queuedTask;

    // Find an available worker
    const workerIndex = this.activeCount % this.poolSize;
    const worker = this.workers[workerIndex];

    if (!worker) {
      reject(new Error('No available worker'));
      clearTimeout(timeout);
      return;
    }

    this.activeCount++;

    const messageHandler = (result: WorkerResult<any>) => {
      clearTimeout(timeout);
      resolve(result as WorkerResult<any>);
      this.activeCount--;
      worker.off('message', messageHandler);
      worker.off('error', errorHandler);
      this.processQueue();
    };

    const errorHandler = (error: Error) => {
      clearTimeout(timeout);
      reject(error);
      this.activeCount--;
      worker.off('message', messageHandler);
      worker.off('error', errorHandler);
      this.processQueue();
    };

    worker.on('message', messageHandler);
    worker.once('error', errorHandler);

    try {
      worker.postMessage(task);
    } catch (error) {
      clearTimeout(timeout);
      reject(error instanceof Error ? error : new Error(String(error)));
      this.activeCount--;
      worker.off('message', messageHandler);
      worker.off('error', errorHandler);
      this.processQueue();
    }
  }

  private handleWorkerError(error: Error): void {
    if (this.queue.length > 0) {
      const queuedTask = this.queue.shift();
      if (queuedTask) {
        clearTimeout(queuedTask.timeout);
        queuedTask.reject(error);
        this.activeCount--;
        this.processQueue();
      }
    }
  }

  /**
   * Terminate all workers
   * @returns Promise that resolves when all workers are terminated
   */
  public async terminate(): Promise<void> {
    this.terminated = true;

    // Reject all queued tasks
    while (this.queue.length > 0) {
      const queuedTask = this.queue.shift();
      if (queuedTask) {
        clearTimeout(queuedTask.timeout);
        queuedTask.reject(new Error('Worker pool terminated'));
      }
    }

    // Terminate all workers
    await Promise.all(
      this.workers.map((worker) =>
        worker
          .terminate()
          .catch((error) =>
            console.warn('Error terminating worker:', error)
          )
      )
    );

    this.cleanup();
  }

  private cleanup(): void {
    this.workers = [];
    this.queue = [];
    this.activeCount = 0;
  }

  /**
   * Get current pool statistics
   */
  public getStats(): {
    poolSize: number;
    activeWorkers: number;
    queuedTasks: number;
    terminated: boolean;
  } {
    return {
      poolSize: this.poolSize,
      activeWorkers: this.activeCount,
      queuedTasks: this.queue.length,
      terminated: this.terminated,
    };
  }
}

/**
 * Global worker pool instance (singleton)
 * Auto-configured based on CPU count
 */
const hardwareConcurrency = Math.max(1, os.cpus().length);

export const workerPool = new WorkerPool(
  Math.min(hardwareConcurrency, 8)
);

/**
 * Graceful shutdown
 */
process.on('exit', async () => {
  if (!workerPool || (workerPool as any).terminated) return;
  try {
    await workerPool.terminate();
  } catch (error) {
    console.error('Error during worker pool shutdown:', error);
  }
});
