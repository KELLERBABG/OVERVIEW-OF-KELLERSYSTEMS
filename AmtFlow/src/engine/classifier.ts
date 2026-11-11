// @ts-ignore
import { classify, RULES } from '../classifier.js';

export interface ClassificationResult {
  docKey: string;
  subject: string;
  sender: string;
  reference: string;
  amount: string | null;
  deadline: string | null;
  daysRemaining: number | null;
  scores: Record<string, number>;
  isInformational: boolean;
}

export function classifyDocument(text: string): ClassificationResult {
  return classify(text);
}

export { RULES };
