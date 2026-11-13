/**
 * Prototype Chain Resolution.
 * Resolves property lookups through the prototype chain following
 * ECMAScript semantics (OrdinaryGetOwnProperty → [[GetPrototypeOf]] loop).
 */

import { MemoryState, ValueRef } from "../types.js";

export interface ProtoResolutionResult {
  /** The object id where the property was found */
  foundOn: number;
  /** The value reference */
  value: ValueRef | null;
  /** Whether the property exists on the chain (even if undefined) */
  hasProperty: boolean;
  /** Number of prototype hops traversed */
  hops: number;
}

export class PrototypeChainResolver {
  constructor(private memory: MemoryState) {}

  /**
   * Resolve a property lookup following the prototype chain.
   * Mirrors ECMAScript OrdinaryGet / GetV.
   */
  resolve(objectId: number, property: string): ProtoResolutionResult {
    let current = objectId;
    let hops = 0;
    const visited = new Set<number>();

    while (current !== -1 && !visited.has(current)) {
      visited.add(current);
      const heapEntry = this.memory.heap.get(current);

      if (heapEntry && heapEntry.has(property)) {
        return {
          foundOn: current,
          value: heapEntry.get(property)!,
          hasProperty: true,
          hops,
        };
      }

      // Follow prototype link
      const proto = this.memory.protoLinks.get(current);
      if (proto === undefined || proto === -1) break;
      current = proto;
      hops++;
    }

    return {
      foundOn: -1,
      value: null,
      hasProperty: false,
      hops,
    };
  }

  /**
   * Check if a property exists anywhere in the chain (HasProperty).
   */
  hasProperty(objectId: number, property: string): boolean {
    return this.resolve(objectId, property).hasProperty;
  }

  /**
   * Get the prototype of an object.
   */
  getPrototype(objectId: number): number | null {
    const proto = this.memory.protoLinks.get(objectId);
    return proto !== undefined ? proto : null;
  }

  /**
   * Walk the full prototype chain from an object to the root.
   */
  walkChain(objectId: number): number[] {
    const chain: number[] = [];
    let current = objectId;
    const visited = new Set<number>();

    while (current !== -1 && !visited.has(current)) {
      visited.add(current);
      chain.push(current);
      const proto = this.memory.protoLinks.get(current);
      if (proto === undefined || proto === -1) break;
      current = proto;
    }

    return chain;
  }

  /**
   * Detect circular prototype chains (should not happen in valid JS,
   * but can occur with Object.create cycles).
   */
  detectCycle(objectId: number): boolean {
    const visited = new Set<number>();
    let current = objectId;

    while (current !== -1) {
      if (visited.has(current)) return true;
      visited.add(current);
      const proto = this.memory.protoLinks.get(current);
      if (proto === undefined || proto === -1) break;
      current = proto;
    }

    return false;
  }
}
