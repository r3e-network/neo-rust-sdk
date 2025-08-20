import React, { useState, useEffect, useRef } from 'react';
import { motion } from 'framer-motion';
import {
  WifiIcon,
  BoltIcon,
  CubeIcon,
  BellIcon,
  FunnelIcon,
  PlayIcon,
  PauseIcon,
  TrashIcon,
  ArrowPathIcon,
} from '@heroicons/react/24/outline';
import { useAppStore } from '../stores/appStore';
import { invoke } from '@tauri-apps/api/core';

interface EventMessage {
  id: string;
  type: 'block' | 'transaction' | 'notification' | 'execution';
  timestamp: number;
  data: any;
}

interface Subscription {
  id: string;
  type: string;
  filter?: string;
  active: boolean;
}

export default function WebSocketMonitor() {
  const { addNotification, currentNetwork } = useAppStore();
  const [connected, setConnected] = useState(false);
  const [connecting, setConnecting] = useState(false);
  const [events, setEvents] = useState<EventMessage[]>([]);
  const [subscriptions, setSubscriptions] = useState<Subscription[]>([]);
  const [wsUrl, setWsUrl] = useState('');
  const [autoScroll, setAutoScroll] = useState(true);
  const [filterType, setFilterType] = useState<string>('all');
  const [showSubscribeModal, setShowSubscribeModal] = useState(false);
  const eventsEndRef = useRef<HTMLDivElement>(null);
  const maxEvents = 100;

  useEffect(() => {
    // Set default WebSocket URL based on current network
    if (currentNetwork) {
      const defaultWsUrl = currentNetwork.rpcUrl
        .replace('http://', 'ws://')
        .replace('https://', 'wss://');
      setWsUrl(defaultWsUrl + '/ws');
    }
  }, [currentNetwork]);

  useEffect(() => {
    if (autoScroll && eventsEndRef.current) {
      eventsEndRef.current.scrollIntoView({ behavior: 'smooth' });
    }
  }, [events, autoScroll]);

  const handleConnect = async () => {
    if (connected) {
      handleDisconnect();
      return;
    }

    setConnecting(true);
    try {
      await invoke('websocket_connect', { url: wsUrl });
      setConnected(true);
      
      // Start listening for events
      startEventListener();
      
      addNotification({
        type: 'success',
        title: 'Connected',
        message: `Connected to WebSocket at ${wsUrl}`,
      });
    } catch (error) {
      console.error('Failed to connect:', error);
      addNotification({
        type: 'error',
        title: 'Connection Failed',
        message: 'Failed to connect to WebSocket endpoint',
      });
    } finally {
      setConnecting(false);
    }
  };

  const handleDisconnect = async () => {
    try {
      await invoke('websocket_disconnect');
      setConnected(false);
      setSubscriptions([]);
      
      addNotification({
        type: 'info',
        title: 'Disconnected',
        message: 'WebSocket connection closed',
      });
    } catch (error) {
      console.error('Failed to disconnect:', error);
    }
  };

  const startEventListener = () => {
    // In a real implementation, this would listen to Tauri events
    // For demo purposes, we'll simulate incoming events
    const interval = setInterval(() => {
      if (!connected) {
        clearInterval(interval);
        return;
      }

      // Simulate random events
      const eventTypes = ['block', 'transaction', 'notification'];
      const randomType = eventTypes[Math.floor(Math.random() * eventTypes.length)];
      
      const newEvent: EventMessage = {
        id: Date.now().toString(),
        type: randomType as any,
        timestamp: Date.now(),
        data: generateMockEventData(randomType),
      };

      setEvents(prev => {
        const updated = [...prev, newEvent];
        // Keep only the last maxEvents
        return updated.slice(-maxEvents);
      });
    }, Math.random() * 3000 + 2000); // Random interval between 2-5 seconds
  };

  const generateMockEventData = (type: string) => {
    switch (type) {
      case 'block':
        return {
          index: Math.floor(Math.random() * 1000000),
          hash: '0x' + Math.random().toString(16).substr(2, 64),
          timestamp: Date.now(),
          transactions: Math.floor(Math.random() * 100),
        };
      case 'transaction':
        return {
          txid: '0x' + Math.random().toString(16).substr(2, 64),
          sender: 'NX8GreRFGFK5wpGMWetpX93HmtrezGogzk',
          receiver: 'NY7GreRFGFK5wpGMWetpX93HmtrezGogzk',
          amount: (Math.random() * 100).toFixed(2),
          asset: Math.random() > 0.5 ? 'NEO' : 'GAS',
        };
      case 'notification':
        return {
          contract: '0x' + Math.random().toString(16).substr(2, 40),
          event: 'Transfer',
          state: {
            from: 'NX8GreRFGFK5wpGMWetpX93HmtrezGogzk',
            to: 'NY7GreRFGFK5wpGMWetpX93HmtrezGogzk',
            amount: Math.floor(Math.random() * 1000),
          },
        };
      default:
        return {};
    }
  };

  const handleSubscribe = async (subscriptionType: string, filter?: string) => {
    if (!connected) {
      addNotification({
        type: 'warning',
        title: 'Not Connected',
        message: 'Please connect to WebSocket first',
      });
      return;
    }

    try {
      await invoke('websocket_subscribe', {
        subscriptionType,
        filter,
      });

      const newSubscription: Subscription = {
        id: Date.now().toString(),
        type: subscriptionType,
        filter,
        active: true,
      };

      setSubscriptions([...subscriptions, newSubscription]);
      
      addNotification({
        type: 'success',
        title: 'Subscribed',
        message: `Subscribed to ${subscriptionType} events`,
      });
    } catch (error) {
      console.error('Failed to subscribe:', error);
      addNotification({
        type: 'error',
        title: 'Subscription Failed',
        message: 'Failed to subscribe to events',
      });
    }
  };

  const handleUnsubscribe = async (subscriptionId: string) => {
    const subscription = subscriptions.find(s => s.id === subscriptionId);
    if (!subscription) return;

    try {
      await invoke('websocket_unsubscribe', {
        subscriptionType: subscription.type,
      });

      setSubscriptions(subscriptions.filter(s => s.id !== subscriptionId));
      
      addNotification({
        type: 'info',
        title: 'Unsubscribed',
        message: `Unsubscribed from ${subscription.type} events`,
      });
    } catch (error) {
      console.error('Failed to unsubscribe:', error);
    }
  };

  const clearEvents = () => {
    setEvents([]);
    addNotification({
      type: 'info',
      title: 'Events Cleared',
      message: 'Event log has been cleared',
    });
  };

  const getEventIcon = (type: string) => {
    switch (type) {
      case 'block':
        return <CubeIcon className='h-5 w-5 text-blue-500' />;
      case 'transaction':
        return <BoltIcon className='h-5 w-5 text-green-500' />;
      case 'notification':
        return <BellIcon className='h-5 w-5 text-purple-500' />;
      default:
        return <WifiIcon className='h-5 w-5 text-gray-500' />;
    }
  };

  const filteredEvents = filterType === 'all' 
    ? events 
    : events.filter(e => e.type === filterType);

  return (
    <div className='space-y-6'>
      {/* Header */}
      <div className='md:flex md:items-center md:justify-between'>
        <div className='flex-1 min-w-0'>
          <h2 className='text-2xl font-bold leading-7 text-gray-900 sm:text-3xl sm:truncate'>
            WebSocket Monitor
          </h2>
          <p className='mt-1 text-sm text-gray-500'>
            Real-time blockchain event monitoring via WebSocket
          </p>
        </div>
        <div className='mt-4 flex space-x-3 md:mt-0 md:ml-4'>
          <button
            onClick={() => setShowSubscribeModal(true)}
            disabled={!connected}
            className='inline-flex items-center px-4 py-2 border border-gray-300 shadow-sm text-sm font-medium rounded-md text-gray-700 bg-white hover:bg-gray-50 disabled:opacity-50'
          >
            <BellIcon className='-ml-1 mr-2 h-5 w-5' />
            Subscribe
          </button>
          <button
            onClick={handleConnect}
            disabled={connecting}
            className={`inline-flex items-center px-4 py-2 border border-transparent shadow-sm text-sm font-medium rounded-md text-white ${
              connected ? 'bg-red-600 hover:bg-red-700' : 'bg-green-600 hover:bg-green-700'
            } disabled:opacity-50`}
          >
            {connecting ? (
              <>
                <ArrowPathIcon className='-ml-1 mr-2 h-5 w-5 animate-spin' />
                Connecting...
              </>
            ) : connected ? (
              <>
                <PauseIcon className='-ml-1 mr-2 h-5 w-5' />
                Disconnect
              </>
            ) : (
              <>
                <PlayIcon className='-ml-1 mr-2 h-5 w-5' />
                Connect
              </>
            )}
          </button>
        </div>
      </div>

      {/* Connection Status */}
      <div className='bg-white shadow rounded-lg p-6'>
        <div className='flex items-center justify-between'>
          <div className='flex items-center'>
            <div className={`h-3 w-3 rounded-full ${connected ? 'bg-green-500' : 'bg-gray-300'} mr-3`} />
            <div>
              <p className='text-sm font-medium text-gray-900'>
                Connection Status: {connected ? 'Connected' : 'Disconnected'}
              </p>
              <p className='text-sm text-gray-500'>{wsUrl || 'No URL configured'}</p>
            </div>
          </div>
          <div className='flex items-center space-x-4'>
            <input
              type='text'
              value={wsUrl}
              onChange={(e) => setWsUrl(e.target.value)}
              disabled={connected}
              className='border-gray-300 rounded-md shadow-sm focus:ring-green-500 focus:border-green-500 text-sm'
              placeholder='wss://testnet1.neo.coz.io:443/ws'
            />
          </div>
        </div>
      </div>

      {/* Active Subscriptions */}
      {subscriptions.length > 0 && (
        <div className='bg-white shadow rounded-lg p-6'>
          <h3 className='text-lg font-medium text-gray-900 mb-4'>
            Active Subscriptions
          </h3>
          <div className='flex flex-wrap gap-2'>
            {subscriptions.map((sub) => (
              <motion.span
                key={sub.id}
                initial={{ opacity: 0, scale: 0.9 }}
                animate={{ opacity: 1, scale: 1 }}
                className='inline-flex items-center px-3 py-1 rounded-full text-sm font-medium bg-green-100 text-green-800'
              >
                {sub.type}
                {sub.filter && ` (${sub.filter.substring(0, 8)}...)`}
                <button
                  onClick={() => handleUnsubscribe(sub.id)}
                  className='ml-2 text-green-600 hover:text-green-700'
                >
                  ×
                </button>
              </motion.span>
            ))}
          </div>
        </div>
      )}

      {/* Event Filter and Controls */}
      <div className='bg-white shadow rounded-lg p-4'>
        <div className='flex items-center justify-between'>
          <div className='flex items-center space-x-4'>
            <FunnelIcon className='h-5 w-5 text-gray-400' />
            <select
              value={filterType}
              onChange={(e) => setFilterType(e.target.value)}
              className='border-gray-300 rounded-md shadow-sm focus:ring-green-500 focus:border-green-500 text-sm'
            >
              <option value='all'>All Events</option>
              <option value='block'>Blocks</option>
              <option value='transaction'>Transactions</option>
              <option value='notification'>Notifications</option>
            </select>
            <span className='text-sm text-gray-500'>
              {filteredEvents.length} events
            </span>
          </div>
          <div className='flex items-center space-x-2'>
            <label className='flex items-center text-sm text-gray-700'>
              <input
                type='checkbox'
                checked={autoScroll}
                onChange={(e) => setAutoScroll(e.target.checked)}
                className='mr-2 rounded border-gray-300 text-green-600 focus:ring-green-500'
              />
              Auto-scroll
            </label>
            <button
              onClick={clearEvents}
              className='inline-flex items-center px-3 py-1 border border-gray-300 shadow-sm text-sm font-medium rounded-md text-gray-700 bg-white hover:bg-gray-50'
            >
              <TrashIcon className='h-4 w-4 mr-1' />
              Clear
            </button>
          </div>
        </div>
      </div>

      {/* Event Log */}
      <div className='bg-white shadow rounded-lg'>
        <div className='px-6 py-4 border-b border-gray-200'>
          <h3 className='text-lg font-medium text-gray-900'>Event Log</h3>
        </div>
        <div className='max-h-96 overflow-y-auto'>
          {filteredEvents.length === 0 ? (
            <div className='px-6 py-8 text-center text-sm text-gray-500'>
              {connected ? 'Waiting for events...' : 'Connect to start receiving events'}
            </div>
          ) : (
            <div className='divide-y divide-gray-200'>
              {filteredEvents.map((event) => (
                <motion.div
                  key={event.id}
                  initial={{ opacity: 0, x: -20 }}
                  animate={{ opacity: 1, x: 0 }}
                  className='px-6 py-3 hover:bg-gray-50'
                >
                  <div className='flex items-start'>
                    <div className='mt-1'>{getEventIcon(event.type)}</div>
                    <div className='ml-3 flex-1'>
                      <div className='flex items-center justify-between'>
                        <p className='text-sm font-medium text-gray-900 capitalize'>
                          {event.type}
                        </p>
                        <span className='text-xs text-gray-500'>
                          {new Date(event.timestamp).toLocaleTimeString()}
                        </span>
                      </div>
                      <pre className='mt-1 text-xs text-gray-600 font-mono bg-gray-50 p-2 rounded overflow-x-auto'>
                        {JSON.stringify(event.data, null, 2)}
                      </pre>
                    </div>
                  </div>
                </motion.div>
              ))}
            </div>
          )}
          <div ref={eventsEndRef} />
        </div>
      </div>

      {/* Subscribe Modal */}
      {showSubscribeModal && (
        <SubscribeModal
          onClose={() => setShowSubscribeModal(false)}
          onSubscribe={handleSubscribe}
        />
      )}
    </div>
  );
}

function SubscribeModal({
  onClose,
  onSubscribe,
}: {
  onClose: () => void;
  onSubscribe: (type: string, filter?: string) => void;
}) {
  const [subscriptionType, setSubscriptionType] = useState('block');
  const [contractFilter, setContractFilter] = useState('');

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    onSubscribe(
      subscriptionType,
      subscriptionType === 'notification' ? contractFilter : undefined
    );
    onClose();
  };

  return (
    <div className='fixed inset-0 z-50 overflow-y-auto'>
      <div className='flex items-center justify-center min-h-screen pt-4 px-4 pb-20 text-center sm:block sm:p-0'>
        <div className='fixed inset-0 transition-opacity' onClick={onClose}>
          <div className='absolute inset-0 bg-gray-500 opacity-75'></div>
        </div>

        <div className='inline-block align-bottom bg-white rounded-lg text-left overflow-hidden shadow-xl transform transition-all sm:my-8 sm:align-middle sm:max-w-lg sm:w-full'>
          <form onSubmit={handleSubmit}>
            <div className='bg-white px-4 pt-5 pb-4 sm:p-6 sm:pb-4'>
              <h3 className='text-lg font-medium text-gray-900 mb-4'>
                Subscribe to Events
              </h3>

              <div className='space-y-4'>
                <div>
                  <label className='block text-sm font-medium text-gray-700'>
                    Event Type
                  </label>
                  <select
                    value={subscriptionType}
                    onChange={(e) => setSubscriptionType(e.target.value)}
                    className='mt-1 block w-full border-gray-300 rounded-md shadow-sm focus:ring-green-500 focus:border-green-500'
                  >
                    <option value='block'>New Blocks</option>
                    <option value='transaction'>Transactions</option>
                    <option value='notification'>Contract Notifications</option>
                    <option value='execution'>Transaction Executions</option>
                  </select>
                </div>

                {subscriptionType === 'notification' && (
                  <div>
                    <label className='block text-sm font-medium text-gray-700'>
                      Contract Hash (optional)
                    </label>
                    <input
                      type='text'
                      value={contractFilter}
                      onChange={(e) => setContractFilter(e.target.value)}
                      className='mt-1 block w-full border-gray-300 rounded-md shadow-sm focus:ring-green-500 focus:border-green-500'
                      placeholder='0x...'
                    />
                    <p className='mt-1 text-xs text-gray-500'>
                      Leave empty to receive all contract notifications
                    </p>
                  </div>
                )}
              </div>
            </div>

            <div className='bg-gray-50 px-4 py-3 sm:px-6 sm:flex sm:flex-row-reverse'>
              <button
                type='submit'
                className='w-full inline-flex justify-center rounded-md border border-transparent shadow-sm px-4 py-2 bg-green-600 text-base font-medium text-white hover:bg-green-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-green-500 sm:ml-3 sm:w-auto sm:text-sm'
              >
                Subscribe
              </button>
              <button
                type='button'
                onClick={onClose}
                className='mt-3 w-full inline-flex justify-center rounded-md border border-gray-300 shadow-sm px-4 py-2 bg-white text-base font-medium text-gray-700 hover:bg-gray-50 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-green-500 sm:mt-0 sm:ml-3 sm:w-auto sm:text-sm'
              >
                Cancel
              </button>
            </div>
          </form>
        </div>
      </div>
    </div>
  );
}