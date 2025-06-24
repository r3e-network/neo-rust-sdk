import React, { useEffect, useState, useCallback } from 'react';
import Link from '@docusaurus/Link';
import useDocusaurusContext from '@docusaurus/useDocusaurusContext';
import Layout from '@theme/Layout';
import CodeBlock from '@theme/CodeBlock';
import styles from './index.module.css';
import clsx from 'clsx';

// Interface for blockchain info
interface BlockchainInfo {
  blockHeight: number;
  blockHash: string;
  timestamp: string;
  transactions: number;
  version: string;
  loading: boolean;
  lastUpdated: number;
}

// Feature data
const features = [
  {
    title: 'Performance Optimized',
    icon: '⚡',
    description: 'Built with Rust\'s performance and safety guarantees for high-throughput blockchain applications.',
  },
  {
    title: 'Comprehensive Security',
    icon: '🔒',
    description: 'State-of-the-art cryptographic implementations with thorough security considerations.',
  },
  {
    title: 'Smart Contract Support',
    icon: '📋',
    description: 'Intuitive interfaces for deploying and interacting with Neo N3 smart contracts.',
  },
  {
    title: 'Wallet Management',
    icon: '💰',
    description: 'Complete wallet functionality with NEP-6 standard support and hardware wallet integration.',
  },
  {
    title: 'Neo X Integration',
    icon: '🔗',
    description: 'Seamless integration with Neo X for EVM compatibility and cross-chain operations.',
  },
  {
    title: 'Developer Friendly',
    icon: '🛠️',
    description: 'Intuitive, well-documented API with type safety and comprehensive examples.',
  },
];

// Stats data
const stats = [
  { label: '100% Rust-Native', value: '100%', description: 'Pure Rust implementation' },
  { label: 'Neo N3 Support', value: 'Full', description: 'Complete Neo N3 compatibility' },
  { label: 'Neo X Ready', value: '✓', description: 'EVM compatibility layer' },
  { label: 'Doc Tests', value: '135/150', description: 'Documentation tests passing' },
];

const exampleCode = `use neo3::prelude::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Connect to Neo N3 TestNet
    let provider = HttpProvider::new("https://testnet1.neo.org:443")?;
    let client = RpcClient::new(provider);
    
    // Get basic blockchain information
    let block_count = client.get_block_count().await?;
    println!("Current block height: {}", block_count);
    
    // Create a new account
    let account = Account::create()?;
    println!("New address: {}", account.get_address());
    
    // Initialize GAS token contract
    let gas_token = GasToken::new(&client);
    let symbol = gas_token.symbol().await?;
    let decimals = gas_token.decimals().await?;
    
    println!("Token: {} with {} decimals", symbol, decimals);
    
    // Check account balance
    let balance = gas_token.balance_of(&account.get_script_hash()).await?;
    println!("Balance: {} {}", balance, symbol);
    
    Ok(())
}`;

function Feature({ title, icon, description }: typeof features[0]) {
  return (
    <div className={clsx('card', styles.feature)}>
      <div className={styles.featureIcon}>{icon}</div>
      <h3 className={styles.featureTitle}>{title}</h3>
      <p className={styles.featureDescription}>{description}</p>
    </div>
  );
}

function Stat({ label, value, description }: typeof stats[0]) {
  return (
    <div className={clsx('card', styles.stat)}>
      <div className={styles.statValue}>{value}</div>
      <div className={styles.statLabel}>{label}</div>
      <div className={styles.statDescription}>{description}</div>
    </div>
  );
}

export default function Home(): JSX.Element {
  const { siteConfig } = useDocusaurusContext();
  
  // State for blockchain info
  const [blockchainInfo, setBlockchainInfo] = useState<BlockchainInfo>({
    blockHeight: 0,
    blockHash: '',
    timestamp: '',
    transactions: 0,
    version: '',
    loading: true,
    lastUpdated: 0
  });

  // Fetch blockchain info
  const fetchBlockchainInfo = useCallback(async () => {
    // List of reliable Neo N3 RPC endpoints to try
    const rpcEndpoints = [
      'https://mainnet1.neo.coz.io:443',
      'https://rpc10.n3.nspcc.ru:10331',
      'https://n3seed1.ngd.network:10332',
      'https://mainnet2.neo.coz.io:443'
    ];

    for (const endpoint of rpcEndpoints) {
      try {
        // Get block count (height)
        const response = await fetch(endpoint, {
          method: 'POST',
          headers: {
            'Content-Type': 'application/json',
          },
          body: JSON.stringify({
            jsonrpc: '2.0',
            id: 1,
            method: 'getblockcount',
            params: []
          })
        });
        
        if (!response.ok) {
          throw new Error(`HTTP ${response.status}`);
        }
        
        const data = await response.json();
        if (data.error) {
          throw new Error(data.error.message);
        }
        
        const blockCount = data.result;
        
        // Get the latest block info
        const blockResponse = await fetch(endpoint, {
          method: 'POST',
          headers: {
            'Content-Type': 'application/json',
          },
          body: JSON.stringify({
            jsonrpc: '2.0',
            id: 2,
            method: 'getblock',
            params: [blockCount - 1, 1]
          })
        });
        
        if (!blockResponse.ok) {
          throw new Error(`HTTP ${blockResponse.status}`);
        }
        
        const blockData = await blockResponse.json();
        if (blockData.error) {
          throw new Error(blockData.error.message);
        }
        
        const block = blockData.result;
        const blockTime = new Date(block.time * 1000).toLocaleString();
        
        // Get version info
        const versionResponse = await fetch(endpoint, {
          method: 'POST',
          headers: {
            'Content-Type': 'application/json',
          },
          body: JSON.stringify({
            jsonrpc: '2.0',
            id: 3,
            method: 'getversion',
            params: []
          })
        });
        
        if (!versionResponse.ok) {
          throw new Error(`HTTP ${versionResponse.status}`);
        }
        
        const versionData = await versionResponse.json();
        if (versionData.error) {
          throw new Error(versionData.error.message);
        }
        
        const version = versionData.result.useragent;
        
        setBlockchainInfo({
          blockHeight: blockCount - 1,
          blockHash: block.hash,
          timestamp: blockTime,
          transactions: block.tx ? block.tx.length : 0,
          version: version,
          loading: false,
          lastUpdated: Date.now()
        });
        
        // If we reach here, the endpoint worked successfully
        console.log(`Successfully fetched blockchain data from ${endpoint}`);
        return;
        
      } catch (error) {
        console.warn(`Failed to fetch from ${endpoint}:`, (error as Error).message);
        // Continue to next endpoint
      }
    }
    
    // If all endpoints failed, set fallback data
    console.error('All Neo N3 RPC endpoints failed');
    setBlockchainInfo({
      blockHeight: 7396359, // Approximate recent block height
      blockHash: '0x...unavailable',
      timestamp: new Date().toLocaleString(),
      transactions: 0,
      version: 'Neo N3 (RPC Unavailable)',
      loading: false,
      lastUpdated: Date.now()
    });
  }, []);

  // Force refresh function
  const refreshBlockchainInfo = () => {
    setBlockchainInfo(prev => ({...prev, loading: true}));
    fetchBlockchainInfo();
  };

  // Fetch blockchain info on mount and periodically
  useEffect(() => {
    fetchBlockchainInfo();
    
    // Update every 15 seconds
    const interval = setInterval(() => {
      fetchBlockchainInfo();
    }, 15000);
    
    return () => {
      clearInterval(interval);
    };
  }, [fetchBlockchainInfo]);

  return (
    <Layout
      title={`${siteConfig.title} - ${siteConfig.tagline}`}
      description="NeoRust v0.4.2 - A production-ready Rust SDK for Neo N3 blockchain development. Build high-performance dApps with type-safe, modern Rust. Features 135 passing documentation tests.">
      
      {/* Hero Section */}
      <header className={clsx('hero hero--primary', styles.heroBanner)}>
        <div className="container">
          <div className={styles.heroContent}>
            <div className={styles.heroLogo}>
              <div className={styles.logoCircle}>
                <span className={styles.logoIcon}>⚡</span>
              </div>
            </div>
            <h1 className={styles.heroTitle}>
              <span className={styles.heroTitlePrimary}>Neo</span>
              <span className={styles.heroTitleSecondary}>Rust SDK</span>
            </h1>
            <p className={styles.heroSubtitle}>
              A production-ready Rust library for building high-performance applications on the Neo N3 blockchain ecosystem. Now with 135 passing documentation tests ensuring code quality and reliability.
            </p>
            
            <div className={styles.qualityBadges}>
              <div className={styles.badge}>
                <span className={styles.badgeIcon}>✅</span>
                <span className={styles.badgeText}>135/150 Doc Tests</span>
              </div>
              <div className={styles.badge}>
                <span className={styles.badgeIcon}>🦀</span>
                <span className={styles.badgeText}>100% Rust Native</span>
              </div>
              <div className={styles.badge}>
                <span className={styles.badgeIcon}>🛡️</span>
                <span className={styles.badgeText}>Production Ready</span>
              </div>
            </div>
            
            <div className={styles.heroButtons}>
              <Link
                className={clsx('btn btn-primary', styles.heroButton)}
                to="/docs/intro">
                Get Started
                <span className={styles.buttonIcon}>→</span>
              </Link>
              <Link
                className={clsx('btn btn-secondary', styles.heroButton)}
                href="https://github.com/R3E-Network/NeoRust">
                <span className={styles.githubIcon}>⭐</span>
                View on GitHub
              </Link>
            </div>
            
            <div className={styles.heroStats}>
              {stats.map((stat, index) => (
                <Stat key={index} {...stat} />
              ))}
            </div>
          </div>
        </div>
      </header>

      <main>
        {/* Blockchain Status Section */}
        <section className={styles.blockchainSection}>
          <div className="container">
            <div className={styles.blockchainCard}>
              <div className={styles.blockchainHeader}>
                <div className={styles.blockchainInfo}>
                  <div className={styles.blockchainIcon}>📊</div>
                  <div>
                    <h3>Neo N3 Blockchain Status</h3>
                    <p>Live network statistics</p>
                  </div>
                </div>
                
                <button 
                  onClick={refreshBlockchainInfo}
                  disabled={blockchainInfo.loading}
                  className={clsx('btn btn-secondary', styles.refreshButton)}
                >
                  {blockchainInfo.loading ? (
                    <>
                      <div className={styles.spinner}></div>
                      Updating...
                    </>
                  ) : (
                    <>
                      <span className={styles.refreshIcon}>🔄</span>
                      Refresh
                    </>
                  )}
                </button>
              </div>
              
              {blockchainInfo.loading && !blockchainInfo.blockHeight ? (
                <div className={styles.loadingState}>
                  <div className={styles.spinner}></div>
                  <span>Loading blockchain data...</span>
                </div>
              ) : (
                <div className={styles.blockchainStats}>
                  <div className={styles.blockchainStat}>
                    <div className={styles.statLabel}>Height</div>
                    <div className={styles.statValue}>{blockchainInfo.blockHeight.toLocaleString()}</div>
                  </div>
                  
                  <div className={styles.blockchainStat}>
                    <div className={styles.statLabel}>Latest Block</div>
                    <div className={styles.blockHash}>
                      <span>{blockchainInfo.blockHash ? `${blockchainInfo.blockHash.substring(0, 6)}...${blockchainInfo.blockHash.substring(blockchainInfo.blockHash.length - 8)}` : ''}</span>
                      {blockchainInfo.blockHash && (
                        <a 
                          href={`https://neo3.neotube.io/block/${blockchainInfo.blockHash}`} 
                          target="_blank" 
                          rel="noopener noreferrer"
                          className={styles.externalLink}
                        >
                          🔗
                        </a>
                      )}
                    </div>
                  </div>
                  
                  <div className={styles.blockchainStat}>
                    <div className={styles.statLabel}>Transactions</div>
                    <div className={styles.statValue}>{blockchainInfo.transactions}</div>
                  </div>
                  
                  <div className={styles.blockchainStat}>
                    <div className={styles.statLabel}>Version</div>
                    <div className={styles.statValue}>{blockchainInfo.version}</div>
                  </div>
                </div>
              )}
              
              {!blockchainInfo.loading && blockchainInfo.lastUpdated > 0 && (
                <div className={styles.lastUpdated}>
                  Last updated: {new Date(blockchainInfo.lastUpdated).toLocaleTimeString()}
                </div>
              )}
            </div>
          </div>
        </section>

        {/* Features Section */}
        <section className={styles.featuresSection}>
          <div className="container">
            <div className={styles.sectionHeader}>
              <h2 className={styles.sectionTitle}>
                <span className="gradient-text">Key Features</span>
              </h2>
              <p className={styles.sectionSubtitle}>
                Built with Rust's performance and safety guarantees for robust blockchain applications
              </p>
            </div>
            
            <div className={styles.featuresGrid}>
              {features.map((props, idx) => (
                <Feature key={idx} {...props} />
              ))}
            </div>
          </div>
        </section>

        {/* Code Example Section */}
        <section className={styles.codeSection}>
          <div className="container">
            <div className={styles.codeContent}>
              <div className={styles.codeInfo}>
                <h2 className={styles.sectionTitle}>
                  <span className="gradient-text">Simple to Use</span>
                </h2>
                <p className={styles.sectionSubtitle}>
                  Write clean, type-safe blockchain code with modern Rust features
                </p>
                
                <div className={styles.codeFeatures}>
                  {[
                    'Type-safe blockchain interactions',
                    'Async/await support for modern codebases',
                    'Comprehensive error handling',
                    'Extensive documentation and examples'
                  ].map((item, index) => (
                    <div key={index} className={styles.codeFeature}>
                      <span className={styles.checkIcon}>✅</span>
                      {item}
                    </div>
                  ))}
                </div>

                <Link
                  className={clsx('btn btn-primary', styles.codeButton)}
                  to="/docs/getting-started/quick-start">
                  View More Examples
                  <span className={styles.buttonIcon}>→</span>
                </Link>
              </div>
              
              <div className={styles.codeExample}>
                <CodeBlock
                  language="rust"
                  title="Getting Started with NeoRust"
                  showLineNumbers>
                  {exampleCode}
                </CodeBlock>
              </div>
            </div>
          </div>
        </section>

        {/* Tools Section */}
        <section className={styles.toolsSection}>
          <div className="container">
            <div className={styles.sectionHeader}>
              <h2 className={styles.sectionTitle}>
                <span className="gradient-text">Complete Toolkit</span>
              </h2>
              <p className={styles.sectionSubtitle}>
                Everything you need for Neo N3 development in one comprehensive suite
              </p>
            </div>
            
            <div className={styles.toolsGrid}>
              <div className={clsx('card', styles.tool)}>
                <div className={styles.toolIcon}>🦀</div>
                <h3 className={styles.toolTitle}>Rust SDK</h3>
                <p className={styles.toolDescription}>
                  Comprehensive Rust library with full Neo N3 support, smart contract interaction, and wallet management.
                </p>
                <Link to="/sdk" className={clsx('btn btn-primary', styles.toolButton)}>
                  Explore SDK
                </Link>
              </div>
              
              <div className={clsx('card', styles.tool)}>
                <div className={styles.toolIcon}>🖥️</div>
                <h3 className={styles.toolTitle}>Desktop GUI</h3>
                <p className={styles.toolDescription}>
                  Modern desktop application built with Tauri for managing wallets, tokens, and blockchain interactions.
                </p>
                <Link to="/gui" className={clsx('btn btn-primary', styles.toolButton)}>
                  View GUI
                </Link>
              </div>
              
              <div className={clsx('card', styles.tool)}>
                <div className={styles.toolIcon}>⌨️</div>
                <h3 className={styles.toolTitle}>CLI Tools</h3>
                <p className={styles.toolDescription}>
                  Command-line interface for developers who prefer terminal-based workflows and automation scripts.
                </p>
                <Link to="/cli" className={clsx('btn btn-primary', styles.toolButton)}>
                  CLI Docs
                </Link>
              </div>
            </div>
          </div>
        </section>

        {/* Getting Started Section */}
        <section className={styles.ctaSection}>
          <div className="container">
            <div className={styles.ctaContent}>
              <h2 className={styles.ctaTitle}>Ready to Build on Neo?</h2>
              <p className={styles.ctaSubtitle}>
                Join the growing community of developers building the future of blockchain with NeoRust v0.4.2
              </p>
              
              <div className={styles.ctaButtons}>
                <Link
                  className={clsx('btn btn-primary', styles.ctaButton)}
                  to="/docs/getting-started/installation">
                  Start Building
                  <span className={styles.buttonIcon}>🚀</span>
                </Link>
                <Link
                  className={clsx('btn btn-secondary', styles.ctaButton)}
                  to="/examples">
                  View Examples
                  <span className={styles.buttonIcon}>📚</span>
                </Link>
              </div>
              
              <div className={styles.ctaNote}>
                <p>
                  <strong>New in v0.4.2:</strong> Production-ready stability with 135 passing documentation tests, enhanced error handling, and comprehensive code quality improvements.
                  <Link to="/blog" className={styles.ctaLink}> Read the release notes →</Link>
                </p>
              </div>
            </div>
          </div>
        </section>
      </main>
    </Layout>
  );
}