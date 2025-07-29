import { create } from 'zustand';
import { devtools } from 'zustand/middleware';
import { invoke } from '@tauri-apps/api/core';

export interface NetworkStatus {
  connected: boolean;
  endpoint: string | null;
  network_type: string;
  block_height: number | null;
  peer_count: number | null;
}

export interface NetworkState {
  status: NetworkStatus;
  connecting: boolean;
  error: string | null;

  // Actions
  connect: (endpoint: string, networkType: string) => Promise<void>;
  disconnect: () => Promise<void>;
  getStatus: () => Promise<void>;
  setStatus: (status: NetworkStatus) => void;
  setConnecting: (connecting: boolean) => void;
  setError: (error: string | null) => void;
}

export const useNetworkStore = create<NetworkState>()(
  devtools(
    (set, get) => ({
      status: {
        connected: false,
        endpoint: null,
        network_type: 'testnet',
        block_height: null,
        peer_count: null,
      },
      connecting: false,
      error: null,

      connect: async (endpoint: string, networkType: string) => {
        set({ connecting: true, error: null });

        try {
          const result = await invoke('connect_to_network', {
            request: { 
              endpoint: endpoint, 
              network_type: networkType 
            },
          });

          console.log('Network connection result:', result);

          // Get updated status after connection
          await get().getStatus();

          set({ connecting: false });
        } catch (error) {
          console.error('Failed to connect to network:', error);
          set({
            connecting: false,
            error:
              error instanceof Error
                ? error.message
                : 'Failed to connect to network',
          });
        }
      },

      disconnect: async () => {
        set({ connecting: true, error: null });

        try {
          await invoke('disconnect_from_network');

          set({
            status: {
              connected: false,
              endpoint: null,
              network_type: 'testnet',
              block_height: null,
              peer_count: null,
            },
            connecting: false,
          });
        } catch (error) {
          console.error('Failed to disconnect from network:', error);
          set({
            connecting: false,
            error:
              error instanceof Error
                ? error.message
                : 'Failed to disconnect from network',
          });
        }
      },

      getStatus: async () => {
        try {
          const result = (await invoke('get_network_status')) as any;

          console.log('Network status result:', result);

          if (result.success && result.data) {
            set({
              status: result.data,
              error: null,
            });
          } else {
            set({
              error: result.error || 'Failed to get network status',
            });
          }
        } catch (error) {
          console.error('Failed to get network status:', error);
          set({
            error:
              error instanceof Error
                ? error.message
                : 'Failed to get network status',
          });
        }
      },

      setStatus: (status: NetworkStatus) => set({ status }),
      setConnecting: (connecting: boolean) => set({ connecting }),
      setError: (error: string | null) => set({ error }),
    }),
    {
      name: 'network-store',
    }
  )
);
