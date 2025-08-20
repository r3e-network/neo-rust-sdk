import React, { useState } from 'react';
import { motion } from 'framer-motion';
import {
  KeyIcon,
  DocumentDuplicateIcon,
  EyeIcon,
  EyeSlashIcon,
  PlusIcon,
  ArrowPathIcon,
  ShieldCheckIcon,
  ExclamationTriangleIcon,
} from '@heroicons/react/24/outline';
import { useAppStore } from '../stores/appStore';
import { invoke } from '@tauri-apps/api/core';

interface DerivedAccount {
  index: number;
  path: string;
  address: string;
  publicKey: string;
  privateKey?: string;
}

export default function HDWallet() {
  const { addNotification } = useAppStore();
  const [mnemonic, setMnemonic] = useState<string>('');
  const [mnemonicVisible, setMnemonicVisible] = useState(false);
  const [accounts, setAccounts] = useState<DerivedAccount[]>([]);
  const [loading, setLoading] = useState(false);
  const [showCreateModal, setShowCreateModal] = useState(false);
  const [showRestoreModal, setShowRestoreModal] = useState(false);
  const [derivationPath, setDerivationPath] = useState("m/44'/888'/0'/0");
  const [accountCount, setAccountCount] = useState(5);

  const handleCreateHDWallet = async () => {
    setLoading(true);
    try {
      const result = await invoke('create_hd_wallet', {
        language: 'english',
        accountCount,
      });
      
      const walletData = result as any;
      setMnemonic(walletData.mnemonic);
      setAccounts(walletData.accounts);
      
      addNotification({
        type: 'success',
        title: 'HD Wallet Created',
        message: 'Your HD wallet has been created successfully. Please save your mnemonic phrase!',
      });
      
      setShowCreateModal(false);
    } catch (error) {
      console.error('Failed to create HD wallet:', error);
      addNotification({
        type: 'error',
        title: 'Error',
        message: 'Failed to create HD wallet',
      });
    } finally {
      setLoading(false);
    }
  };

  const handleRestoreHDWallet = async (mnemonicPhrase: string) => {
    setLoading(true);
    try {
      const result = await invoke('restore_hd_wallet', {
        mnemonic: mnemonicPhrase,
        accountCount,
        derivationPath,
      });
      
      const walletData = result as any;
      setMnemonic(mnemonicPhrase);
      setAccounts(walletData.accounts);
      
      addNotification({
        type: 'success',
        title: 'HD Wallet Restored',
        message: 'Your HD wallet has been restored successfully',
      });
      
      setShowRestoreModal(false);
    } catch (error) {
      console.error('Failed to restore HD wallet:', error);
      addNotification({
        type: 'error',
        title: 'Error',
        message: 'Failed to restore HD wallet. Please check your mnemonic phrase.',
      });
    } finally {
      setLoading(false);
    }
  };

  const deriveMoreAccounts = async () => {
    if (!mnemonic) {
      addNotification({
        type: 'warning',
        title: 'No Wallet',
        message: 'Please create or restore an HD wallet first',
      });
      return;
    }

    setLoading(true);
    try {
      const startIndex = accounts.length;
      const result = await invoke('derive_accounts', {
        mnemonic,
        startIndex,
        count: 5,
        derivationPath,
      });
      
      const newAccounts = result as DerivedAccount[];
      setAccounts([...accounts, ...newAccounts]);
      
      addNotification({
        type: 'success',
        title: 'Accounts Derived',
        message: `${newAccounts.length} new accounts derived successfully`,
      });
    } catch (error) {
      console.error('Failed to derive accounts:', error);
      addNotification({
        type: 'error',
        title: 'Error',
        message: 'Failed to derive more accounts',
      });
    } finally {
      setLoading(false);
    }
  };

  const copyToClipboard = (text: string, label: string) => {
    navigator.clipboard.writeText(text);
    addNotification({
      type: 'success',
      title: 'Copied',
      message: `${label} copied to clipboard`,
    });
  };

  if (!mnemonic) {
    return (
      <div className='space-y-6'>
        <div className='md:flex md:items-center md:justify-between'>
          <div className='flex-1 min-w-0'>
            <h2 className='text-2xl font-bold leading-7 text-gray-900 sm:text-3xl sm:truncate'>
              HD Wallet (BIP-39/44)
            </h2>
            <p className='mt-1 text-sm text-gray-500'>
              Hierarchical Deterministic wallet with mnemonic phrase backup
            </p>
          </div>
        </div>

        <div className='bg-white shadow rounded-lg p-8'>
          <div className='text-center'>
            <KeyIcon className='mx-auto h-12 w-12 text-gray-400' />
            <h3 className='mt-2 text-lg font-medium text-gray-900'>
              No HD Wallet
            </h3>
            <p className='mt-1 text-sm text-gray-500'>
              Create a new HD wallet or restore from a mnemonic phrase
            </p>
            <div className='mt-6 flex justify-center space-x-3'>
              <button
                onClick={() => setShowCreateModal(true)}
                className='inline-flex items-center px-4 py-2 border border-transparent shadow-sm text-sm font-medium rounded-md text-white bg-green-600 hover:bg-green-700'
              >
                <PlusIcon className='-ml-1 mr-2 h-5 w-5' />
                Create HD Wallet
              </button>
              <button
                onClick={() => setShowRestoreModal(true)}
                className='inline-flex items-center px-4 py-2 border border-gray-300 shadow-sm text-sm font-medium rounded-md text-gray-700 bg-white hover:bg-gray-50'
              >
                <ArrowPathIcon className='-ml-1 mr-2 h-5 w-5' />
                Restore from Mnemonic
              </button>
            </div>
          </div>
        </div>
      </div>
    );
  }

  return (
    <div className='space-y-6'>
      <div className='md:flex md:items-center md:justify-between'>
        <div className='flex-1 min-w-0'>
          <h2 className='text-2xl font-bold leading-7 text-gray-900 sm:text-3xl sm:truncate'>
            HD Wallet Management
          </h2>
          <p className='mt-1 text-sm text-gray-500'>
            Manage your hierarchical deterministic wallet and derived accounts
          </p>
        </div>
        <div className='mt-4 flex space-x-3 md:mt-0 md:ml-4'>
          <button
            onClick={deriveMoreAccounts}
            disabled={loading}
            className='inline-flex items-center px-4 py-2 border border-transparent shadow-sm text-sm font-medium rounded-md text-white bg-green-600 hover:bg-green-700 disabled:opacity-50'
          >
            <PlusIcon className='-ml-1 mr-2 h-5 w-5' />
            Derive More Accounts
          </button>
        </div>
      </div>

      {/* Mnemonic Phrase Card */}
      <motion.div
        initial={{ opacity: 0, y: 20 }}
        animate={{ opacity: 1, y: 0 }}
        className='bg-yellow-50 border border-yellow-200 rounded-lg p-6'
      >
        <div className='flex items-start'>
          <ExclamationTriangleIcon className='h-6 w-6 text-yellow-600 mt-1' />
          <div className='ml-3 flex-1'>
            <h3 className='text-sm font-medium text-yellow-800'>
              Mnemonic Phrase (Recovery Seed)
            </h3>
            <div className='mt-2'>
              <div className='bg-white rounded-md p-4 border border-yellow-300'>
                <div className='flex items-center justify-between mb-2'>
                  <span className='text-xs text-yellow-600 uppercase tracking-wide'>
                    24-word recovery phrase
                  </span>
                  <button
                    onClick={() => setMnemonicVisible(!mnemonicVisible)}
                    className='text-yellow-600 hover:text-yellow-700'
                  >
                    {mnemonicVisible ? (
                      <EyeSlashIcon className='h-5 w-5' />
                    ) : (
                      <EyeIcon className='h-5 w-5' />
                    )}
                  </button>
                </div>
                <div className='font-mono text-sm text-gray-900'>
                  {mnemonicVisible ? (
                    <div className='grid grid-cols-3 gap-2'>
                      {mnemonic.split(' ').map((word, index) => (
                        <span key={index} className='bg-gray-100 px-2 py-1 rounded'>
                          {index + 1}. {word}
                        </span>
                      ))}
                    </div>
                  ) : (
                    '••• ••• ••• ••• ••• ••• ••• ••• ••• ••• ••• ••• ••• ••• ••• ••• ••• ••• ••• ••• ••• ••• ••• •••'
                  )}
                </div>
                {mnemonicVisible && (
                  <button
                    onClick={() => copyToClipboard(mnemonic, 'Mnemonic phrase')}
                    className='mt-3 text-sm text-yellow-600 hover:text-yellow-700 flex items-center'
                  >
                    <DocumentDuplicateIcon className='h-4 w-4 mr-1' />
                    Copy to clipboard
                  </button>
                )}
              </div>
              <p className='mt-2 text-xs text-yellow-700'>
                <ShieldCheckIcon className='inline h-4 w-4 mr-1' />
                Never share your mnemonic phrase. Anyone with this phrase can access your funds.
              </p>
            </div>
          </div>
        </div>
      </motion.div>

      {/* Derivation Path */}
      <div className='bg-white shadow rounded-lg p-6'>
        <h3 className='text-lg font-medium text-gray-900 mb-4'>
          Derivation Path
        </h3>
        <div className='flex items-center space-x-4'>
          <input
            type='text'
            value={derivationPath}
            onChange={(e) => setDerivationPath(e.target.value)}
            className='flex-1 border-gray-300 rounded-md shadow-sm focus:ring-green-500 focus:border-green-500'
            placeholder="m/44'/888'/0'/0"
          />
          <span className='text-sm text-gray-500'>
            BIP-44 Standard Path for Neo
          </span>
        </div>
      </div>

      {/* Derived Accounts */}
      <div className='bg-white shadow rounded-lg'>
        <div className='px-6 py-4 border-b border-gray-200'>
          <h3 className='text-lg font-medium text-gray-900'>
            Derived Accounts ({accounts.length})
          </h3>
        </div>
        <div className='divide-y divide-gray-200'>
          {accounts.map((account) => (
            <motion.div
              key={account.index}
              initial={{ opacity: 0, x: -20 }}
              animate={{ opacity: 1, x: 0 }}
              transition={{ delay: account.index * 0.05 }}
              className='px-6 py-4 hover:bg-gray-50'
            >
              <div className='flex items-center justify-between'>
                <div className='flex-1'>
                  <div className='flex items-center'>
                    <span className='inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium bg-green-100 text-green-800'>
                      #{account.index}
                    </span>
                    <span className='ml-3 text-sm font-medium text-gray-900'>
                      {account.path}/{account.index}
                    </span>
                  </div>
                  <div className='mt-2'>
                    <p className='text-sm text-gray-500'>Address:</p>
                    <div className='flex items-center mt-1'>
                      <code className='text-xs font-mono text-gray-800 bg-gray-100 px-2 py-1 rounded'>
                        {account.address}
                      </code>
                      <button
                        onClick={() => copyToClipboard(account.address, 'Address')}
                        className='ml-2 text-gray-400 hover:text-gray-600'
                      >
                        <DocumentDuplicateIcon className='h-4 w-4' />
                      </button>
                    </div>
                  </div>
                </div>
                <div className='ml-4'>
                  <button className='text-sm text-green-600 hover:text-green-500'>
                    View Details
                  </button>
                </div>
              </div>
            </motion.div>
          ))}
        </div>
        {accounts.length === 0 && (
          <div className='px-6 py-8 text-center text-sm text-gray-500'>
            No accounts derived yet
          </div>
        )}
      </div>

      {/* Modals */}
      {showCreateModal && (
        <CreateHDWalletModal
          onClose={() => setShowCreateModal(false)}
          onCreate={handleCreateHDWallet}
          loading={loading}
          accountCount={accountCount}
          setAccountCount={setAccountCount}
        />
      )}

      {showRestoreModal && (
        <RestoreHDWalletModal
          onClose={() => setShowRestoreModal(false)}
          onRestore={handleRestoreHDWallet}
          loading={loading}
        />
      )}
    </div>
  );
}

// Modal Components
function CreateHDWalletModal({
  onClose,
  onCreate,
  loading,
  accountCount,
  setAccountCount,
}: {
  onClose: () => void;
  onCreate: () => void;
  loading: boolean;
  accountCount: number;
  setAccountCount: (count: number) => void;
}) {
  return (
    <div className='fixed inset-0 z-50 overflow-y-auto'>
      <div className='flex items-center justify-center min-h-screen pt-4 px-4 pb-20 text-center sm:block sm:p-0'>
        <div className='fixed inset-0 transition-opacity' onClick={onClose}>
          <div className='absolute inset-0 bg-gray-500 opacity-75'></div>
        </div>

        <div className='inline-block align-bottom bg-white rounded-lg text-left overflow-hidden shadow-xl transform transition-all sm:my-8 sm:align-middle sm:max-w-lg sm:w-full'>
          <div className='bg-white px-4 pt-5 pb-4 sm:p-6 sm:pb-4'>
            <h3 className='text-lg font-medium text-gray-900 mb-4'>
              Create HD Wallet
            </h3>

            <div className='space-y-4'>
              <div>
                <label className='block text-sm font-medium text-gray-700'>
                  Number of accounts to derive
                </label>
                <input
                  type='number'
                  value={accountCount}
                  onChange={(e) => setAccountCount(parseInt(e.target.value) || 1)}
                  min='1'
                  max='100'
                  className='mt-1 block w-full border-gray-300 rounded-md shadow-sm focus:ring-green-500 focus:border-green-500'
                />
              </div>

              <div className='bg-blue-50 rounded-lg p-4'>
                <p className='text-sm text-blue-700'>
                  <ShieldCheckIcon className='inline h-4 w-4 mr-1' />
                  A 24-word mnemonic phrase will be generated. This is your wallet backup - keep it safe!
                </p>
              </div>
            </div>
          </div>

          <div className='bg-gray-50 px-4 py-3 sm:px-6 sm:flex sm:flex-row-reverse'>
            <button
              type='button'
              onClick={onCreate}
              disabled={loading}
              className='w-full inline-flex justify-center rounded-md border border-transparent shadow-sm px-4 py-2 bg-green-600 text-base font-medium text-white hover:bg-green-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-green-500 sm:ml-3 sm:w-auto sm:text-sm disabled:opacity-50'
            >
              {loading ? 'Creating...' : 'Create Wallet'}
            </button>
            <button
              type='button'
              onClick={onClose}
              className='mt-3 w-full inline-flex justify-center rounded-md border border-gray-300 shadow-sm px-4 py-2 bg-white text-base font-medium text-gray-700 hover:bg-gray-50 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-green-500 sm:mt-0 sm:ml-3 sm:w-auto sm:text-sm'
            >
              Cancel
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}

function RestoreHDWalletModal({
  onClose,
  onRestore,
  loading,
}: {
  onClose: () => void;
  onRestore: (mnemonic: string) => void;
  loading: boolean;
}) {
  const [mnemonic, setMnemonic] = useState('');

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    const words = mnemonic.trim().split(/\s+/);
    if (words.length !== 12 && words.length !== 24) {
      alert('Please enter a valid 12 or 24 word mnemonic phrase');
      return;
    }
    onRestore(mnemonic.trim());
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
                Restore HD Wallet
              </h3>

              <div className='space-y-4'>
                <div>
                  <label className='block text-sm font-medium text-gray-700'>
                    Mnemonic Phrase (12 or 24 words)
                  </label>
                  <textarea
                    value={mnemonic}
                    onChange={(e) => setMnemonic(e.target.value)}
                    rows={4}
                    className='mt-1 block w-full border-gray-300 rounded-md shadow-sm focus:ring-green-500 focus:border-green-500'
                    placeholder='Enter your mnemonic phrase words separated by spaces'
                    required
                  />
                </div>
              </div>
            </div>

            <div className='bg-gray-50 px-4 py-3 sm:px-6 sm:flex sm:flex-row-reverse'>
              <button
                type='submit'
                disabled={loading}
                className='w-full inline-flex justify-center rounded-md border border-transparent shadow-sm px-4 py-2 bg-green-600 text-base font-medium text-white hover:bg-green-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-green-500 sm:ml-3 sm:w-auto sm:text-sm disabled:opacity-50'
              >
                {loading ? 'Restoring...' : 'Restore Wallet'}
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