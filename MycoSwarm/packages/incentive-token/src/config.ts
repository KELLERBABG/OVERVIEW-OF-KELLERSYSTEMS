export interface TokenConfig {
  chainId: string;
  tokenAddress: string;
  marketAddress: string;
  rpcUrl: string;
}

export const defaultConfig: TokenConfig = {
  chainId: '31337',
  tokenAddress: '',
  marketAddress: '',
  rpcUrl: 'http://localhost:8545',
};