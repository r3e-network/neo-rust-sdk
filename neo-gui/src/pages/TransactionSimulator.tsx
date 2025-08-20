import React, { useState } from 'react';
import { motion } from 'framer-motion';
import {
  BeakerIcon,
  PlayIcon,
  DocumentTextIcon,
  CalculatorIcon,
  CheckCircleIcon,
  XCircleIcon,
  InformationCircleIcon,
  ClipboardDocumentIcon,
} from '@heroicons/react/24/outline';
import { useAppStore } from '../stores/appStore';
import { invoke } from '@tauri-apps/api/core';

interface SimulationResult {
  state: 'HALT' | 'FAULT';
  gasConsumed: string;
  systemFee: string;
  networkFee: string;
  totalFee: string;
  stack: any[];
  notifications: Array<{
    contract: string;
    eventName: string;
    state: any;
  }>;
  logs: string[];
  exception?: string;
}

interface ScriptTemplate {
  name: string;
  description: string;
  script: string;
  signers: string[];
}

const scriptTemplates: ScriptTemplate[] = [
  {
    name: 'NEO Transfer',
    description: 'Transfer NEO tokens between addresses',
    script: '0c14{recipient}0c14{sender}53c56b6c766b00527ac46c766b51527ac46161681953797374656d2e52756e74696d652e436865636b5769746e657373',
    signers: ['{sender_address}'],
  },
  {
    name: 'GAS Claim',
    description: 'Claim accumulated GAS rewards',
    script: '0c14{address}51c3066e656f17c10667617343616c63756c6174654761736672656541c36c766b00527ac46203006c766b00c3616c7566',
    signers: ['{address}'],
  },
  {
    name: 'Contract Call',
    description: 'Generic smart contract invocation',
    script: '0c14{contract_hash}0c08{method}0c00{params}41c36c766b00527ac46203006c766b00c3616c7566',
    signers: ['{caller_address}'],
  },
];

export default function TransactionSimulator() {
  const { addNotification, currentWallet } = useAppStore();
  const [script, setScript] = useState('');
  const [signers, setSigners] = useState<string[]>(['']);
  const [selectedTemplate, setSelectedTemplate] = useState<string>('');
  const [simulating, setSimulating] = useState(false);
  const [result, setResult] = useState<SimulationResult | null>(null);
  const [showAdvanced, setShowAdvanced] = useState(false);
  const [scriptFormat, setScriptFormat] = useState<'hex' | 'base64'>('hex');

  const handleSimulate = async () => {
    if (!script.trim()) {
      addNotification({
        type: 'warning',
        title: 'Missing Script',
        message: 'Please provide a transaction script to simulate',
      });
      return;
    }

    if (signers.filter(s => s.trim()).length === 0) {
      addNotification({
        type: 'warning',
        title: 'Missing Signers',
        message: 'Please provide at least one signer address',
      });
      return;
    }

    setSimulating(true);
    try {
      const response = await invoke('simulate_transaction', {
        script: script.trim(),
        signers: signers.filter(s => s.trim()),
        format: scriptFormat,
      });

      const simulationResult = response as SimulationResult;
      setResult(simulationResult);

      if (simulationResult.state === 'HALT') {
        addNotification({
          type: 'success',
          title: 'Simulation Successful',
          message: `Transaction would succeed. Estimated fee: ${simulationResult.totalFee} GAS`,
        });
      } else {
        addNotification({
          type: 'error',
          title: 'Simulation Failed',
          message: simulationResult.exception || 'Transaction would fail during execution',
        });
      }
    } catch (error) {
      console.error('Simulation failed:', error);
      addNotification({
        type: 'error',
        title: 'Simulation Error',
        message: 'Failed to simulate transaction',
      });
    } finally {
      setSimulating(false);
    }
  };

  const handleTemplateSelect = (template: ScriptTemplate) => {
    setSelectedTemplate(template.name);
    setScript(template.script);
    setSigners(template.signers);
    
    addNotification({
      type: 'info',
      title: 'Template Loaded',
      message: `Loaded ${template.name} template. Replace placeholder values before simulating.`,
    });
  };

  const addSigner = () => {
    setSigners([...signers, '']);
  };

  const updateSigner = (index: number, value: string) => {
    const updated = [...signers];
    updated[index] = value;
    setSigners(updated);
  };

  const removeSigner = (index: number) => {
    setSigners(signers.filter((_, i) => i !== index));
  };

  const copyToClipboard = (text: string, label: string) => {
    navigator.clipboard.writeText(text);
    addNotification({
      type: 'success',
      title: 'Copied',
      message: `${label} copied to clipboard`,
    });
  };

  const formatGasAmount = (amount: string) => {
    const num = parseFloat(amount);
    if (isNaN(num)) return '0';
    return num.toFixed(8);
  };

  return (
    <div className='space-y-6'>
      {/* Header */}
      <div className='md:flex md:items-center md:justify-between'>
        <div className='flex-1 min-w-0'>
          <h2 className='text-2xl font-bold leading-7 text-gray-900 sm:text-3xl sm:truncate'>
            Transaction Simulator
          </h2>
          <p className='mt-1 text-sm text-gray-500'>
            Simulate transactions before sending to estimate gas costs and verify execution
          </p>
        </div>
      </div>

      <div className='grid grid-cols-1 lg:grid-cols-2 gap-6'>
        {/* Input Section */}
        <div className='space-y-6'>
          {/* Script Templates */}
          <div className='bg-white shadow rounded-lg p-6'>
            <h3 className='text-lg font-medium text-gray-900 mb-4'>
              Script Templates
            </h3>
            <div className='space-y-2'>
              {scriptTemplates.map((template) => (
                <button
                  key={template.name}
                  onClick={() => handleTemplateSelect(template)}
                  className={`w-full text-left px-4 py-3 rounded-lg border ${
                    selectedTemplate === template.name
                      ? 'border-green-500 bg-green-50'
                      : 'border-gray-200 hover:bg-gray-50'
                  }`}
                >
                  <div className='flex items-center justify-between'>
                    <div>
                      <p className='font-medium text-gray-900'>{template.name}</p>
                      <p className='text-sm text-gray-500'>{template.description}</p>
                    </div>
                    <DocumentTextIcon className='h-5 w-5 text-gray-400' />
                  </div>
                </button>
              ))}
            </div>
          </div>

          {/* Transaction Script */}
          <div className='bg-white shadow rounded-lg p-6'>
            <div className='flex items-center justify-between mb-4'>
              <h3 className='text-lg font-medium text-gray-900'>
                Transaction Script
              </h3>
              <select
                value={scriptFormat}
                onChange={(e) => setScriptFormat(e.target.value as 'hex' | 'base64')}
                className='text-sm border-gray-300 rounded-md'
              >
                <option value='hex'>Hex</option>
                <option value='base64'>Base64</option>
              </select>
            </div>
            <textarea
              value={script}
              onChange={(e) => setScript(e.target.value)}
              rows={6}
              className='w-full border-gray-300 rounded-md shadow-sm focus:ring-green-500 focus:border-green-500 font-mono text-sm'
              placeholder={scriptFormat === 'hex' ? '0x...' : 'Base64 encoded script...'}
            />
            <p className='mt-2 text-xs text-gray-500'>
              Enter the transaction script in {scriptFormat} format
            </p>
          </div>

          {/* Signers */}
          <div className='bg-white shadow rounded-lg p-6'>
            <div className='flex items-center justify-between mb-4'>
              <h3 className='text-lg font-medium text-gray-900'>
                Transaction Signers
              </h3>
              <button
                onClick={addSigner}
                className='text-sm text-green-600 hover:text-green-500'
              >
                + Add Signer
              </button>
            </div>
            <div className='space-y-2'>
              {signers.map((signer, index) => (
                <div key={index} className='flex items-center space-x-2'>
                  <input
                    type='text'
                    value={signer}
                    onChange={(e) => updateSigner(index, e.target.value)}
                    className='flex-1 border-gray-300 rounded-md shadow-sm focus:ring-green-500 focus:border-green-500 text-sm'
                    placeholder='Address or script hash'
                  />
                  {signers.length > 1 && (
                    <button
                      onClick={() => removeSigner(index)}
                      className='text-red-600 hover:text-red-700'
                    >
                      ×
                    </button>
                  )}
                </div>
              ))}
            </div>
            {currentWallet && (
              <button
                onClick={() => updateSigner(0, currentWallet.address)}
                className='mt-2 text-xs text-green-600 hover:text-green-500'
              >
                Use current wallet address
              </button>
            )}
          </div>

          {/* Advanced Options */}
          <div className='bg-white shadow rounded-lg p-6'>
            <button
              onClick={() => setShowAdvanced(!showAdvanced)}
              className='flex items-center justify-between w-full'
            >
              <h3 className='text-lg font-medium text-gray-900'>
                Advanced Options
              </h3>
              <span className='text-gray-400'>
                {showAdvanced ? '−' : '+'}
              </span>
            </button>
            {showAdvanced && (
              <div className='mt-4 space-y-4'>
                <div>
                  <label className='block text-sm font-medium text-gray-700'>
                    System Fee Multiplier
                  </label>
                  <input
                    type='number'
                    defaultValue='1.0'
                    step='0.1'
                    min='1.0'
                    className='mt-1 block w-full border-gray-300 rounded-md shadow-sm focus:ring-green-500 focus:border-green-500'
                  />
                </div>
                <div>
                  <label className='block text-sm font-medium text-gray-700'>
                    Network Fee Priority
                  </label>
                  <select className='mt-1 block w-full border-gray-300 rounded-md shadow-sm focus:ring-green-500 focus:border-green-500'>
                    <option>Low</option>
                    <option>Normal</option>
                    <option>High</option>
                  </select>
                </div>
              </div>
            )}
          </div>

          {/* Simulate Button */}
          <button
            onClick={handleSimulate}
            disabled={simulating || !script.trim()}
            className='w-full inline-flex justify-center items-center px-4 py-3 border border-transparent shadow-sm text-base font-medium rounded-md text-white bg-green-600 hover:bg-green-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-green-500 disabled:opacity-50'
          >
            {simulating ? (
              <>
                <BeakerIcon className='animate-pulse -ml-1 mr-3 h-5 w-5' />
                Simulating...
              </>
            ) : (
              <>
                <PlayIcon className='-ml-1 mr-3 h-5 w-5' />
                Simulate Transaction
              </>
            )}
          </button>
        </div>

        {/* Results Section */}
        <div className='space-y-6'>
          {result ? (
            <>
              {/* Execution Result */}
              <motion.div
                initial={{ opacity: 0, y: 20 }}
                animate={{ opacity: 1, y: 0 }}
                className='bg-white shadow rounded-lg p-6'
              >
                <h3 className='text-lg font-medium text-gray-900 mb-4'>
                  Simulation Result
                </h3>
                <div className='space-y-4'>
                  <div className='flex items-center justify-between p-4 rounded-lg bg-gray-50'>
                    <span className='text-sm font-medium text-gray-700'>
                      Execution State
                    </span>
                    <div className='flex items-center'>
                      {result.state === 'HALT' ? (
                        <>
                          <CheckCircleIcon className='h-5 w-5 text-green-500 mr-2' />
                          <span className='text-green-600 font-medium'>SUCCESS</span>
                        </>
                      ) : (
                        <>
                          <XCircleIcon className='h-5 w-5 text-red-500 mr-2' />
                          <span className='text-red-600 font-medium'>FAILED</span>
                        </>
                      )}
                    </div>
                  </div>

                  {result.exception && (
                    <div className='p-4 rounded-lg bg-red-50 border border-red-200'>
                      <div className='flex'>
                        <InformationCircleIcon className='h-5 w-5 text-red-400 mt-0.5' />
                        <div className='ml-3'>
                          <h4 className='text-sm font-medium text-red-800'>
                            Exception
                          </h4>
                          <p className='mt-1 text-sm text-red-700'>
                            {result.exception}
                          </p>
                        </div>
                      </div>
                    </div>
                  )}
                </div>
              </motion.div>

              {/* Gas Costs */}
              <motion.div
                initial={{ opacity: 0, y: 20 }}
                animate={{ opacity: 1, y: 0 }}
                transition={{ delay: 0.1 }}
                className='bg-white shadow rounded-lg p-6'
              >
                <div className='flex items-center mb-4'>
                  <CalculatorIcon className='h-5 w-5 text-gray-400 mr-2' />
                  <h3 className='text-lg font-medium text-gray-900'>
                    Gas Estimation
                  </h3>
                </div>
                <div className='space-y-3'>
                  <div className='flex justify-between py-2 border-b border-gray-100'>
                    <span className='text-sm text-gray-600'>GAS Consumed</span>
                    <span className='text-sm font-medium text-gray-900'>
                      {formatGasAmount(result.gasConsumed)} GAS
                    </span>
                  </div>
                  <div className='flex justify-between py-2 border-b border-gray-100'>
                    <span className='text-sm text-gray-600'>System Fee</span>
                    <span className='text-sm font-medium text-gray-900'>
                      {formatGasAmount(result.systemFee)} GAS
                    </span>
                  </div>
                  <div className='flex justify-between py-2 border-b border-gray-100'>
                    <span className='text-sm text-gray-600'>Network Fee</span>
                    <span className='text-sm font-medium text-gray-900'>
                      {formatGasAmount(result.networkFee)} GAS
                    </span>
                  </div>
                  <div className='flex justify-between py-2'>
                    <span className='text-sm font-medium text-gray-900'>
                      Total Fee
                    </span>
                    <span className='text-base font-semibold text-green-600'>
                      {formatGasAmount(result.totalFee)} GAS
                    </span>
                  </div>
                </div>
              </motion.div>

              {/* Notifications */}
              {result.notifications.length > 0 && (
                <motion.div
                  initial={{ opacity: 0, y: 20 }}
                  animate={{ opacity: 1, y: 0 }}
                  transition={{ delay: 0.2 }}
                  className='bg-white shadow rounded-lg p-6'
                >
                  <h3 className='text-lg font-medium text-gray-900 mb-4'>
                    Contract Notifications ({result.notifications.length})
                  </h3>
                  <div className='space-y-3'>
                    {result.notifications.map((notif, index) => (
                      <div key={index} className='p-3 bg-gray-50 rounded-lg'>
                        <div className='flex items-center justify-between mb-2'>
                          <span className='text-sm font-medium text-gray-900'>
                            {notif.eventName}
                          </span>
                          <button
                            onClick={() => copyToClipboard(notif.contract, 'Contract hash')}
                            className='text-gray-400 hover:text-gray-600'
                          >
                            <ClipboardDocumentIcon className='h-4 w-4' />
                          </button>
                        </div>
                        <p className='text-xs text-gray-600 font-mono'>
                          Contract: {notif.contract}
                        </p>
                        <pre className='mt-2 text-xs text-gray-700 bg-white p-2 rounded overflow-x-auto'>
                          {JSON.stringify(notif.state, null, 2)}
                        </pre>
                      </div>
                    ))}
                  </div>
                </motion.div>
              )}

              {/* Logs */}
              {result.logs.length > 0 && (
                <motion.div
                  initial={{ opacity: 0, y: 20 }}
                  animate={{ opacity: 1, y: 0 }}
                  transition={{ delay: 0.3 }}
                  className='bg-white shadow rounded-lg p-6'
                >
                  <h3 className='text-lg font-medium text-gray-900 mb-4'>
                    Execution Logs
                  </h3>
                  <div className='bg-gray-900 rounded-lg p-4 max-h-64 overflow-y-auto'>
                    <pre className='text-xs text-green-400 font-mono'>
                      {result.logs.join('\n')}
                    </pre>
                  </div>
                </motion.div>
              )}

              {/* Stack Result */}
              {result.stack.length > 0 && (
                <motion.div
                  initial={{ opacity: 0, y: 20 }}
                  animate={{ opacity: 1, y: 0 }}
                  transition={{ delay: 0.4 }}
                  className='bg-white shadow rounded-lg p-6'
                >
                  <h3 className='text-lg font-medium text-gray-900 mb-4'>
                    Stack Result
                  </h3>
                  <pre className='text-xs text-gray-700 bg-gray-50 p-4 rounded overflow-x-auto'>
                    {JSON.stringify(result.stack, null, 2)}
                  </pre>
                </motion.div>
              )}
            </>
          ) : (
            <div className='bg-white shadow rounded-lg p-8'>
              <div className='text-center'>
                <BeakerIcon className='mx-auto h-12 w-12 text-gray-400' />
                <h3 className='mt-2 text-sm font-medium text-gray-900'>
                  No Simulation Results
                </h3>
                <p className='mt-1 text-sm text-gray-500'>
                  Configure and run a simulation to see results here
                </p>
              </div>
            </div>
          )}
        </div>
      </div>
    </div>
  );
}