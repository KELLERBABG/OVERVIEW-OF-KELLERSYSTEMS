import type { ExecutorConfig } from './config.js';

export class Sandbox {
  private worker: Worker | null = null;
  private iframe: HTMLIFrameElement | null = null;

  constructor(private config: ExecutorConfig) {
    if (config.sandboxType === 'worker' && typeof Worker !== 'undefined') {
      this.worker = this.createSandboxedWorker();
    } else if (config.sandboxType === 'iframe' && typeof document !== 'undefined') {
      this.iframe = this.createSandboxedIframe();
    }
  }

  private createSandboxedWorker(): Worker {
    const blob = new Blob(
      [
        `
      self.onmessage = async (e) => {
        const { bytecode, input, maxMemory } = e.data;
        try {
          const importObj = {
            env: {
              memory: new WebAssembly.Memory({ initial: maxMemory >> 16, maximum: maxMemory >> 16 }),
              abort: () => { throw new Error('abort'); },
            }
          };
          const module = await WebAssembly.instantiate(bytecode, importObj);
          const result = module.instance.exports.main(input);
          self.postMessage({ type: 'result', data: result });
        } catch (err) {
          self.postMessage({ type: 'error', data: (err as Error).message });
        }
      };
    `,
      ],
      { type: 'application/javascript' },
    );
    return new Worker(URL.createObjectURL(blob));
  }

  private createSandboxedIframe(): HTMLIFrameElement {
    const iframe = document.createElement('iframe');
    iframe.sandbox = 'allow-scripts';
    iframe.style.display = 'none';
    document.body.appendChild(iframe);
    return iframe;
  }

  async compile(bytecode: Uint8Array, memoryBudget: number): Promise<WebAssembly.Instance> {
    if (this.worker) {
      return new Promise((resolve, reject) => {
        this.worker!.onmessage = (e) => {
          if (e.data.type === 'result') resolve(e.data.instance);
          else reject(new Error(e.data.data));
        };
        this.worker!.postMessage({ bytecode, memoryBudget });
      });
    }
    const module = await WebAssembly.compile(bytecode as unknown as BufferSource);
    return WebAssembly.instantiate(module, {
      env: {
        memory: new WebAssembly.Memory({ initial: memoryBudget >> 16 }),
        abort: () => {
          throw new Error('abort');
        },
      },
    });
  }

  async run(
    instance: WebAssembly.Instance,
    exportName: string,
    args: unknown[],
    signal: { aborted: boolean },
  ): Promise<unknown> {
    const func = instance.exports[exportName as keyof WebAssembly.Exports] as
      | CallableFunction
      | undefined;
    if (!func) throw new Error(`Export "${exportName}" not found`);

    if (signal.aborted) throw new Error('Execution aborted');
    return func(...args);
  }

  terminate(): void {
    this.worker?.terminate();
    if (this.iframe) {
      this.iframe.remove();
    }
  }
}