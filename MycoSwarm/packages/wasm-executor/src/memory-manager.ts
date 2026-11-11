export class MemoryManager {
  private allocated = 0;
  private readonly limit: number;

  constructor(limit: number) {
    this.limit = limit;
  }

  allocate(bytes: number): boolean {
    if (this.allocated + bytes > this.limit) return false;
    this.allocated += bytes;
    return true;
  }

  release(bytes: number): void {
    this.allocated = Math.max(0, this.allocated - bytes);
  }

  get used(): number {
    return this.allocated;
  }

  get available(): number {
    return this.limit - this.allocated;
  }

  get utilization(): number {
    return this.allocated / this.limit;
  }

  canAllocate(bytes: number): boolean {
    return this.allocated + bytes <= this.limit;
  }

  reset(): void {
    this.allocated = 0;
  }
}