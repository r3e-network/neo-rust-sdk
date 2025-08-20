import React, { useState } from 'react';
import { Outlet, Link, useLocation } from 'react-router-dom';
import { motion, AnimatePresence } from 'framer-motion';
import {
  HomeIcon,
  WalletIcon,
  CubeIcon,
  WrenchScrewdriverIcon,
  Cog6ToothIcon,
  ChartBarIcon,
  BellIcon,
  UserCircleIcon,
  Bars3Icon,
  XMarkIcon,
  KeyIcon,
  WifiIcon,
  BeakerIcon,
} from '@heroicons/react/24/outline';
import { useAppStore } from '../stores/appStore';
import { useNetworkStore } from '../stores/networkStore';

const navigation = [
  { name: 'Dashboard', href: '/', icon: HomeIcon },
  { name: 'Wallet', href: '/wallet', icon: WalletIcon },
  { name: 'HD Wallet', href: '/hd-wallet', icon: KeyIcon },
  { name: 'NFTs', href: '/nft', icon: CubeIcon },
  { name: 'Tools', href: '/tools', icon: WrenchScrewdriverIcon },
  { name: 'WebSocket', href: '/websocket', icon: WifiIcon },
  { name: 'Simulator', href: '/simulator', icon: BeakerIcon },
  { name: 'Analytics', href: '/analytics', icon: ChartBarIcon },
  { name: 'Settings', href: '/settings', icon: Cog6ToothIcon },
];

export default function Layout() {
  const [sidebarOpen, setSidebarOpen] = useState(false);
  const location = useLocation();
  const { walletConnected, notifications } = useAppStore();
  const { status: networkStatus } = useNetworkStore();

  const currentPage = navigation.find(item => item.href === location.pathname);

  return (
    <div className='min-h-screen bg-gray-50'>
      {/* Mobile sidebar */}
      <AnimatePresence>
        {sidebarOpen && (
          <>
            <motion.div
              initial={{ opacity: 0 }}
              animate={{ opacity: 1 }}
              exit={{ opacity: 0 }}
              className='fixed inset-0 z-40 lg:hidden'
              onClick={() => setSidebarOpen(false)}
            >
              <div className='absolute inset-0 bg-gray-600 opacity-75' />
            </motion.div>
            <motion.div
              initial={{ x: -300 }}
              animate={{ x: 0 }}
              exit={{ x: -300 }}
              transition={{ type: 'spring', damping: 25, stiffness: 200 }}
              className='fixed inset-y-0 left-0 z-50 w-64 bg-white shadow-xl lg:hidden'
            >
              <Sidebar onClose={() => setSidebarOpen(false)} />
            </motion.div>
          </>
        )}
      </AnimatePresence>

      {/* Desktop sidebar */}
      <div className='hidden lg:fixed lg:inset-y-0 lg:flex lg:w-64 lg:flex-col'>
        <Sidebar />
      </div>

      {/* Main content */}
      <div className='lg:pl-64'>
        {/* Top navigation */}
        <div className='sticky top-0 z-30 flex h-16 shrink-0 items-center gap-x-4 border-b border-gray-200 bg-white px-4 shadow-sm sm:gap-x-6 sm:px-6 lg:px-8'>
          <button
            type='button'
            className='-m-2.5 p-2.5 text-gray-700 lg:hidden'
            onClick={() => setSidebarOpen(true)}
          >
            <Bars3Icon className='h-6 w-6' />
          </button>

          {/* Separator */}
          <div className='h-6 w-px bg-gray-200 lg:hidden' />

          <div className='flex flex-1 gap-x-4 self-stretch lg:gap-x-6'>
            <div className='flex items-center gap-x-4 lg:gap-x-6'>
              {/* Page title */}
              <h1 className='text-xl font-semibold text-gray-900'>
                {currentPage?.name || 'Neo N3 Wallet'}
              </h1>

              {/* Network indicator */}
              <div className='flex items-center gap-x-2'>
                <div
                  className={`h-2 w-2 rounded-full ${
                    networkStatus.connected ? 'bg-green-500' : 'bg-red-500'
                  }`}
                />
                <span className='text-sm text-gray-600 capitalize'>
                  {networkStatus.connected
                    ? networkStatus.network_type
                    : 'Disconnected'}
                </span>
              </div>
            </div>

            <div className='flex flex-1 justify-end gap-x-4 lg:gap-x-6'>
              {/* Wallet status */}
              <div className='flex items-center gap-x-2'>
                <div
                  className={`h-2 w-2 rounded-full ${
                    walletConnected ? 'bg-green-500' : 'bg-red-500'
                  }`}
                />
                <span className='text-sm text-gray-600'>
                  {walletConnected ? 'Connected' : 'Disconnected'}
                </span>
              </div>

              {/* Notifications */}
              <button className='relative -m-2.5 p-2.5 text-gray-400 hover:text-gray-500'>
                <BellIcon className='h-6 w-6' />
                {notifications.length > 0 && (
                  <span className='absolute -top-1 -right-1 h-4 w-4 rounded-full bg-red-500 text-xs text-white flex items-center justify-center'>
                    {notifications.length}
                  </span>
                )}
              </button>

              {/* Profile */}
              <button className='-m-2.5 p-2.5 text-gray-400 hover:text-gray-500'>
                <UserCircleIcon className='h-6 w-6' />
              </button>
            </div>
          </div>
        </div>

        {/* Page content */}
        <main className='py-6'>
          <div className='px-4 sm:px-6 lg:px-8'>
            <motion.div
              key={location.pathname}
              initial={{ opacity: 0, y: 20 }}
              animate={{ opacity: 1, y: 0 }}
              exit={{ opacity: 0, y: -20 }}
              transition={{ duration: 0.2 }}
            >
              <Outlet />
            </motion.div>
          </div>
        </main>
      </div>
    </div>
  );
}

function Sidebar({ onClose }: { onClose?: () => void }) {
  const location = useLocation();

  return (
    <div className='flex grow flex-col gap-y-5 overflow-y-auto bg-white px-6 pb-4 border-r border-gray-200'>
      <div className='flex h-16 shrink-0 items-center justify-between'>
        <div className='flex items-center gap-x-3'>
          <div className='h-8 w-8 rounded-lg bg-gradient-to-br from-green-400 to-green-600 flex items-center justify-center'>
            <span className='text-white font-bold text-sm'>N3</span>
          </div>
          <span className='text-xl font-bold text-gray-900'>Neo Wallet</span>
        </div>
        {onClose && (
          <button
            type='button'
            className='-m-2.5 p-2.5 text-gray-700 lg:hidden'
            onClick={onClose}
          >
            <XMarkIcon className='h-6 w-6' />
          </button>
        )}
      </div>

      <nav className='flex flex-1 flex-col'>
        <ul role='list' className='flex flex-1 flex-col gap-y-7'>
          <li>
            <ul role='list' className='-mx-2 space-y-1'>
              {navigation.map(item => {
                const isActive = location.pathname === item.href;
                return (
                  <li key={item.name}>
                    <Link
                      to={item.href}
                      className={`group flex gap-x-3 rounded-md p-2 text-sm leading-6 font-semibold transition-colors ${
                        isActive
                          ? 'bg-green-50 text-green-700'
                          : 'text-gray-700 hover:text-green-700 hover:bg-green-50'
                      }`}
                      onClick={onClose}
                    >
                      <item.icon
                        className={`h-6 w-6 shrink-0 transition-colors ${
                          isActive
                            ? 'text-green-700'
                            : 'text-gray-400 group-hover:text-green-700'
                        }`}
                      />
                      {item.name}
                    </Link>
                  </li>
                );
              })}
            </ul>
          </li>

          {/* Footer */}
          <li className='mt-auto'>
            <div className='rounded-lg bg-gray-50 p-4'>
              <div className='flex items-center gap-x-3'>
                <div className='h-10 w-10 rounded-full bg-gradient-to-br from-green-400 to-green-600 flex items-center justify-center'>
                  <span className='text-white font-bold text-sm'>N3</span>
                </div>
                <div>
                  <p className='text-sm font-medium text-gray-900'>
                    Neo N3 SDK
                  </p>
                  <p className='text-xs text-gray-600'>v0.4.1</p>
                </div>
              </div>
            </div>
          </li>
        </ul>
      </nav>
    </div>
  );
}
