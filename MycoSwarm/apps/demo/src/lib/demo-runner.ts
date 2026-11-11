import { runPrimeSearchDemo } from '../demos/prime-search.js';
import { runMandelbrotDemo } from '../demos/mandelbrot.js';
import type { TaskDistributor } from '@fungal/task-distribution';

export class DemoRunner {
  private distributor: TaskDistributor & { mesh: { peerId: string } };
  private results: DemoResult[] = [];

  constructor(distributor: TaskDistributor & { mesh: { peerId: string } }) {
    this.distributor = distributor;
  }

  async runDemo(name: string, params: Record<string, unknown>): Promise<DemoResult> {
    let result: DemoResult;

    switch (name) {
      case 'prime-search':
        result = await runPrimeSearchDemo(this.distributor, params['limit'] as number);
        break;
      case 'mandelbrot':
        result = await runMandelbrotDemo(
          this.distributor,
          params['width'] as number,
          params['height'] as number,
        );
        break;
      default:
        throw new Error(`Unknown demo: ${name}`);
    }

    this.results.push(result);
    return result;
  }

  getResults(): DemoResult[] {
    return [...this.results];
  }

  clearResults(): void {
    this.results = [];
  }
}

export interface DemoResult {
  taskId: string;
  executionTime: number;
  [key: string]: unknown;
}