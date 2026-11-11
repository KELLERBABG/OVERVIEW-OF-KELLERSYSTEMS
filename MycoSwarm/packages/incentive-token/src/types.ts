export interface TaskPayment {
  requester: string;
  totalReward: bigint;
  unitReward: bigint;
  unitsCompleted: number;
  finalized: boolean;
}

export interface Receipt {
  unitId: string;
  amount: bigint;
  signature: string;
}