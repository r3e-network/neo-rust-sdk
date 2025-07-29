document.addEventListener('DOMContentLoaded', function() {
  // Loading animation
  setTimeout(function() {
    document.querySelector('.loading').classList.add('hidden');
  }, 500);

  // Theme toggle
  const themeToggle = document.querySelector('.theme-toggle');
  const darkIcon = document.querySelector('.dark-icon');
  const lightIcon = document.querySelector('.light-icon');
  
  themeToggle.addEventListener('click', function() {
    document.body.classList.toggle('light-theme');
    
    if (document.body.classList.contains('light-theme')) {
      darkIcon.style.display = 'none';
      lightIcon.style.display = 'block';
      localStorage.setItem('theme', 'light');
    } else {
      darkIcon.style.display = 'block';
      lightIcon.style.display = 'none';
      localStorage.setItem('theme', 'dark');
    }
  });
  
  // Check for saved theme
  const savedTheme = localStorage.getItem('theme');
  if (savedTheme === 'light') {
    document.body.classList.add('light-theme');
    darkIcon.style.display = 'none';
    lightIcon.style.display = 'block';
  }
  
  // Scroll animation
  const scrollAnimated = document.querySelectorAll('.scroll-animated');
  
  const observerOptions = {
    threshold: 0.1
  };
  
  const observer = new IntersectionObserver(function(entries) {
    entries.forEach(entry => {
      if (entry.isIntersecting) {
        entry.target.classList.add('visible');
      }
    });
  }, observerOptions);
  
  scrollAnimated.forEach(element => {
    observer.observe(element);
  });
  
  // Smooth scroll for navigation links
  document.querySelectorAll('a[href^="#"]').forEach(anchor => {
    anchor.addEventListener('click', function (e) {
      e.preventDefault();
      
      document.querySelector(this.getAttribute('href')).scrollIntoView({
        behavior: 'smooth'
      });
    });
  });

  // Mobile menu toggle
  const mobileMenuButton = document.querySelector('.mobile-menu-button');
  const navLinks = document.querySelector('.nav-links');
  
  if (mobileMenuButton) {
    mobileMenuButton.addEventListener('click', function() {
      navLinks.classList.toggle('mobile-active');
    });
  }

  // Code copy functionality
  document.querySelectorAll('.copy-button').forEach(button => {
    button.addEventListener('click', function() {
      const codeElement = this.parentElement.querySelector('pre');
      const code = codeElement.textContent;
      
      navigator.clipboard.writeText(code).then(() => {
        const originalText = this.textContent;
        this.textContent = 'Copied!';
        
        setTimeout(() => {
          this.textContent = originalText;
        }, 2000);
      });
    });
  });

  // Active navigation highlight
  function setActiveNavLink() {
    const currentPath = window.location.pathname;
    document.querySelectorAll('.nav-link').forEach(link => {
      const linkPath = link.getAttribute('href');
      if (linkPath === currentPath || (linkPath !== '/' && currentPath.includes(linkPath))) {
        link.classList.add('active');
      } else {
        link.classList.remove('active');
      }
    });
  }
  
  setActiveNavLink();

  // API example tabs
  document.querySelectorAll('.api-example-tab').forEach(tab => {
    tab.addEventListener('click', function() {
      const tabGroup = this.parentElement;
      const tabContainer = tabGroup.parentElement;
      const tabId = this.getAttribute('data-tab');
      
      // Remove active class from all tabs in this group
      tabGroup.querySelectorAll('.api-example-tab').forEach(t => {
        t.classList.remove('active');
      });
      
      // Remove active class from all content in this container
      tabContainer.querySelectorAll('.api-example-content').forEach(c => {
        c.classList.remove('active');
      });
      
      // Add active class to clicked tab and corresponding content
      this.classList.add('active');
      tabContainer.querySelector(`.api-example-content[data-tab="${tabId}"]`).classList.add('active');
    });
  });

  // Table of contents highlight on scroll
  function highlightTOC() {
    const headings = document.querySelectorAll('.docs-content h2, .docs-content h3');
    const tocLinks = document.querySelectorAll('.docs-toc a');
    
    if (headings.length === 0 || tocLinks.length === 0) return;
    
    let currentActiveIndex = 0;
    
    headings.forEach((heading, index) => {
      const rect = heading.getBoundingClientRect();
      if (rect.top <= 100) {
        currentActiveIndex = index;
      }
    });
    
    tocLinks.forEach(link => link.classList.remove('active'));
    tocLinks[currentActiveIndex].classList.add('active');
  }
  
  if (document.querySelector('.docs-toc')) {
    window.addEventListener('scroll', highlightTOC);
    highlightTOC();
  }

  // Search functionality using Netlify Function
  const searchInput = document.querySelector('.search-input');
  const searchResults = document.querySelector('.search-results');
  
  if (searchInput) {
    let searchTimeout;
    
    // Show search results when input is focused
    searchInput.addEventListener('focus', function() {
      if (this.value.trim().length > 0) {
        searchResults.style.display = 'block';
      }
    });
    
    // Hide search results when clicking outside
    document.addEventListener('click', function(e) {
      if (!searchInput.contains(e.target) && !searchResults.contains(e.target)) {
        searchResults.style.display = 'none';
      }
    });
    
    // Perform search when typing
    searchInput.addEventListener('input', function() {
      clearTimeout(searchTimeout);
      
      const query = this.value.trim();
      
      if (query.length === 0) {
        searchResults.style.display = 'none';
        return;
      }
      
      // Add loading indicator
      searchResults.innerHTML = '<div class="search-loading">Searching...</div>';
      searchResults.style.display = 'block';
      
      // Debounce the search to avoid too many requests
      searchTimeout = setTimeout(() => {
        // Validate query to prevent XSS
        if (query.length > 100 || !/^[a-zA-Z0-9\s\-_\.]+$/.test(query)) {
          searchResults.innerHTML = '<div class="search-error">Invalid search query</div>';
          return;
        }
        
        // Call the Netlify Function for search
        // nosemgrep: javascript.lang.security.detect-eval-with-expression.detect-eval-with-expression
        fetch(`/.netlify/functions/search?query=${encodeURIComponent(query)}`)
          .then(response => response.json())
          .then(data => {
            if (data.results && data.results.length > 0) {
              // Clear search results safely
              while (searchResults.firstChild) {
                searchResults.removeChild(searchResults.firstChild);
              }
              
              data.results.forEach(result => {
                const resultElem = document.createElement('div');
                resultElem.className = 'search-result';
                
                // Create elements securely to prevent XSS
                const sectionDiv = document.createElement('div');
                sectionDiv.className = 'search-result-section';
                sectionDiv.textContent = result.section || '';
                
                const titleDiv = document.createElement('div');
                titleDiv.className = 'search-result-title';
                titleDiv.textContent = result.title || '';
                
                const previewDiv = document.createElement('div');
                previewDiv.className = 'search-result-preview';
                previewDiv.textContent = result.preview || '';
                
                resultElem.appendChild(sectionDiv);
                resultElem.appendChild(titleDiv);
                resultElem.appendChild(previewDiv);
                
                resultElem.addEventListener('click', function() {
                  // Validate URL before navigation to prevent XSS
                  if (result.url && (result.url.startsWith('/') || result.url.startsWith('#'))) {
                    window.location.href = result.url;
                  }
                });
                
                searchResults.appendChild(resultElem);
              });
            } else {
              searchResults.innerHTML = '<div class="search-no-results">No results found</div>';
            }
          })
          .catch(error => {
            console.error('Search error:', error);
            searchResults.innerHTML = '<div class="search-error">Error performing search</div>';
          });
      }, 300);
    });
  }

  // Newsletter subscription using Netlify Function
  const newsletterForm = document.getElementById('newsletter-form');
  
  if (newsletterForm) {
    newsletterForm.addEventListener('submit', function(e) {
      e.preventDefault();
      
      const emailInput = this.querySelector('input[type="email"]');
      const submitButton = this.querySelector('button[type="submit"]');
      const statusMessage = this.querySelector('.newsletter-status');
      
      if (!emailInput || !submitButton) return;
      
      const email = emailInput.value.trim();
      
      if (!email) {
        if (statusMessage) {
          statusMessage.textContent = 'Please enter your email address';
          statusMessage.classList.add('newsletter-error');
          statusMessage.classList.remove('newsletter-success');
        }
        return;
      }
      
      // Disable form while submitting
      emailInput.disabled = true;
      submitButton.disabled = true;
      submitButton.textContent = 'Subscribing...';
      
      // Call the Netlify Function for newsletter subscription
      fetch('/.netlify/functions/newsletter', {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
        },
        body: JSON.stringify({ email })
      })
        .then(response => response.json())
        .then(data => {
          if (data.success) {
            if (statusMessage) {
              statusMessage.textContent = data.message;
              statusMessage.classList.add('newsletter-success');
              statusMessage.classList.remove('newsletter-error');
            }
            emailInput.value = '';
          } else {
            if (statusMessage) {
              statusMessage.textContent = data.error || 'Error subscribing to newsletter';
              statusMessage.classList.add('newsletter-error');
              statusMessage.classList.remove('newsletter-success');
            }
          }
        })
        .catch(error => {
          console.error('Newsletter subscription error:', error);
          if (statusMessage) {
            statusMessage.textContent = 'Error subscribing to newsletter';
            statusMessage.classList.add('newsletter-error');
            statusMessage.classList.remove('newsletter-success');
          }
        })
        .finally(() => {
          // Re-enable form
          emailInput.disabled = false;
          submitButton.disabled = false;
          submitButton.textContent = 'Subscribe';
        });
    });
  }

  // Blockchain status using Netlify Function
  const blockchainStatusElement = document.getElementById('blockchain-status');
  
  if (blockchainStatusElement) {
    // Call the Netlify Function to get blockchain status
    fetch('/.netlify/functions/blockchain-status')
      .then(response => response.json())
      .then(data => {
        if (data.status) {
          const status = data.status;
          
          // Format numbers with commas
          const formatNumber = (num) => {
            return num.toString().replace(/\B(?=(\d{3})+(?!\d))/g, ",");
          };
          
          // Format the blockchain status display
          blockchainStatusElement.innerHTML = `
            <div class="blockchain-status-item">
              <div class="blockchain-status-label">Height</div>
              <div class="blockchain-status-value">${formatNumber(status.height)}</div>
            </div>
            <div class="blockchain-status-item">
              <div class="blockchain-status-label">Latest Block</div>
              <div class="blockchain-status-value blockchain-status-hash" title="${status.latestBlockHash}">${status.latestBlockHash.substring(0, 8)}...${status.latestBlockHash.substring(status.latestBlockHash.length - 8)}</div>
            </div>
            <div class="blockchain-status-item">
              <div class="blockchain-status-label">Transactions</div>
              <div class="blockchain-status-value">${formatNumber(status.latestBlockTx)}</div>
            </div>
            <div class="blockchain-status-item">
              <div class="blockchain-status-label">Version</div>
              <div class="blockchain-status-value">${status.version}</div>
            </div>
          `;
          
          blockchainStatusElement.classList.add('blockchain-status-loaded');
        }
      })
      .catch(error => {
        console.error('Error fetching blockchain status:', error);
        blockchainStatusElement.innerHTML = '<div class="blockchain-status-error">Error fetching blockchain status</div>';
      });
  }
});

// Initialize playground if on the playground page
function initPlayground() {
  const playgroundEditor = document.getElementById('playground-editor');
  const runButton = document.getElementById('run-code');
  const outputElement = document.getElementById('playground-output');
  const exampleSelect = document.getElementById('example-select');
  
  if (!playgroundEditor || !runButton || !outputElement) return;
  
  // Example code snippets
  const examples = {
    'wallet': 'use neo3::prelude::*;\n\nfn main() -> Result<()> {\n    // Create a new wallet\n    let wallet = Wallet::new();\n    \n    // Print the wallet address\n    println!("New wallet address: {}", wallet.address());\n    \n    Ok(())\n}',
    'transaction': 'use neo3::prelude::*;\n\nasync fn transfer_neo() -> Result<()> {\n    let wallet = Wallet::load("wallet.json", "password")?;\n    let account = wallet.default_account()?;\n    \n    // Connect to Neo node\n    let client = NeoClient::connect_to_testnet().await?;\n    \n    // Create a transaction to transfer NEO\n    let tx = TransactionBuilder::new()\n        .version(0)\n        .nonce(1234)\n        .valid_until_block(client.get_block_count().await? + 100)\n        .sender(account.address())\n        .script(script_builder::build_transfer_script(\n            "neo", \n            account.address(),\n            "NbnjKGMBJzJ6j5PHeYhjJDaQ5Vy5UYu4Fv",\n            100000000 // 1 NEO\n        ))\n        .sign(account)\n        .build();\n    \n    // Send the transaction\n    let result = client.send_transaction(tx).await?;\n    println!("Transaction sent: {}", result);\n    \n    Ok(())\n}',
    'contract': 'use neo3::prelude::*;\n\nasync fn invoke_contract() -> Result<()> {\n    let wallet = Wallet::load("wallet.json", "password")?;\n    let account = wallet.default_account()?;\n    \n    // Connect to Neo node\n    let client = NeoClient::connect_to_testnet().await?;\n    \n    // Invoke contract method\n    let result = client\n        .invoke_function(\n            "0xd2a4cff31913016155e38e474a2c06d08be276cf",\n            "transfer",\n            [\n                ContractParameter::hash160(account.address()),\n                ContractParameter::hash160("NbnjKGMBJzJ6j5PHeYhjJDaQ5Vy5UYu4Fv"),\n                ContractParameter::integer(100),\n                ContractParameter::any(None)\n            ],\n            account,\n        )\n        .await?;\n    \n    println!("Transaction: {}", result.tx_id);\n    \n    Ok(())\n}'
  };
  
  // Initialize Monaco Editor
  if (typeof monaco !== 'undefined') {
    const editor = monaco.editor.create(playgroundEditor, {
      value: examples.wallet,
      language: 'rust',
      theme: document.body.classList.contains('light-theme') ? 'vs-light' : 'vs-dark',
      minimap: { enabled: false },
      scrollBeyondLastLine: false,
      automaticLayout: true,
      fontSize: 14,
      fontFamily: '"JetBrains Mono", monospace',
      tabSize: 4,
      insertSpaces: true
    });
    
    // Change example code when select changes
    exampleSelect.addEventListener('change', function() {
      const exampleCode = examples[this.value];
      editor.setValue(exampleCode);
    });
    
    // Run code button (call Netlify Function)
    runButton.addEventListener('click', function() {
      const code = editor.getValue();
      
      if (!code.trim()) {
        outputElement.textContent = "Error: Code is empty";
        return;
      }
      
      // Show loading state
      outputElement.textContent = "Running...";
      runButton.disabled = true;
      
      // Call the Netlify Function to execute the code
      fetch('/.netlify/functions/execute-code', {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
        },
        body: JSON.stringify({ code })
      })
        .then(response => response.json())
        .then(data => {
          if (data.output) {
            outputElement.textContent = data.output;
          } else if (data.error) {
            outputElement.textContent = `Error: ${data.error}`;
          } else {
            outputElement.textContent = "Unknown response from server";
          }
        })
        .catch(error => {
          console.error('Code execution error:', error);
          outputElement.textContent = `Error: ${error.message || 'Unknown error occurred'}`;
        })
        .finally(() => {
          runButton.disabled = false;
        });
    });
    
    // Theme change handler
    const themeToggle = document.querySelector('.theme-toggle');
    if (themeToggle) {
      themeToggle.addEventListener('click', function() {
        editor.updateOptions({
          theme: document.body.classList.contains('light-theme') ? 'vs-light' : 'vs-dark'
        });
      });
    }
  }
}

// Call playground init if monaco is loaded
if (document.getElementById('playground-editor')) {
  if (typeof monaco !== 'undefined') {
    initPlayground();
  } else {
    window.addEventListener('load', function() {
      if (typeof monaco !== 'undefined') {
        initPlayground();
      }
    });
  }
}

// Particles effect on homepage (if tsparticles is loaded)
const particlesContainer = document.getElementById('particles-container');
if (particlesContainer && typeof tsParticles !== 'undefined') {
  tsParticles.load('particles-container', {
    particles: {
      number: {
        value: 80,
        density: {
          enable: true,
          value_area: 800
        }
      },
      color: {
        value: "#4CFFB3"
      },
      shape: {
        type: "circle",
        stroke: {
          width: 0,
          color: "#000000"
        }
      },
      opacity: {
        value: 0.3,
        random: false,
        anim: {
          enable: false,
          speed: 1,
          opacity_min: 0.1,
          sync: false
        }
      },
      size: {
        value: 3,
        random: true,
        anim: {
          enable: false,
          speed: 40,
          size_min: 0.1,
          sync: false
        }
      },
      line_linked: {
        enable: true,
        distance: 150,
        color: "#4CFFB3",
        opacity: 0.2,
        width: 1
      },
      move: {
        enable: true,
        speed: 2,
        direction: "none",
        random: false,
        straight: false,
        out_mode: "out",
        bounce: false,
        attract: {
          enable: false,
          rotateX: 600,
          rotateY: 1200
        }
      }
    },
    interactivity: {
      detect_on: "canvas",
      events: {
        onhover: {
          enable: true,
          mode: "grab"
        },
        onclick: {
          enable: true,
          mode: "push"
        },
        resize: true
      },
      modes: {
        grab: {
          distance: 140,
          line_linked: {
            opacity: 0.5
          }
        },
        push: {
          particles_nb: 4
        }
      }
    },
    retina_detect: true
  });
}