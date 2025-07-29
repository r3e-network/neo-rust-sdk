// @ts-check
// Note: type annotations allow type checking and IDEs autocompletion

const {themes} = require('prism-react-renderer');
const lightCodeTheme = themes.github;
const darkCodeTheme = themes.dracula;

/** @type {import('@docusaurus/types').Config} */
const config = {
  title: 'NeoRust v0.4.3',
  tagline: 'Production-ready Neo N3 blockchain development toolkit built in Rust',
  favicon: 'img/favicon.svg',

  // Set the production url of your site here
  url: 'https://neorust.netlify.app',
  // Set the /<baseUrl>/ pathname under which your site is served
  // For GitHub pages deployment, it is often '/<projectName>/'
  baseUrl: '/',

  // GitHub pages deployment config.
  // If you aren't using GitHub pages, you don't need these.
  organizationName: 'R3E-Network', // Usually your GitHub org/user name.
  projectName: 'NeoRust', // Usually your repo name.

  onBrokenLinks: 'warn',
  onBrokenMarkdownLinks: 'warn',

  // Even if you don't use internalization, you can use this field to set useful
  // metadata like html lang. For example, if your site is Chinese, you may want
  // to replace "en" with "zh-Hans".
  i18n: {
    defaultLocale: 'en',
    locales: ['en'],
  },

  presets: [
    [
      'classic',
      /** @type {import('@docusaurus/preset-classic').Options} */
      ({
        docs: {
          sidebarPath: require.resolve('./sidebars.js'),
          // Please change this to your repo.
          // Remove this to remove the "edit this page" links.
          editUrl: 'https://github.com/R3E-Network/NeoRust/tree/main/website/',
          showLastUpdateAuthor: true,
          showLastUpdateTime: true,
          includeCurrentVersion: true,
          versions: {
            current: {
              label: 'v0.4.3',
              path: '',
            },
          },
        },
        blog: {
          showReadingTime: true,
          blogTitle: 'NeoRust Blog',
          blogDescription: 'Latest updates, tutorials, and insights about NeoRust SDK development',
          postsPerPage: 'ALL',
          // Please change this to your repo.
          // Remove this to remove the "edit this page" links.
          editUrl: 'https://github.com/R3E-Network/NeoRust/tree/main/website/',
        },
        theme: {
          customCss: require.resolve('./src/css/custom.css'),
        },
        sitemap: {
          changefreq: 'weekly',
          priority: 0.5,
          ignorePatterns: ['/tags/**'],
          filename: 'sitemap.xml',
        },
      }),
    ],
  ],

  themeConfig:
    /** @type {import('@docusaurus/preset-classic').ThemeConfig} */
    ({
      // Replace with your project's social card
      image: 'img/neorust-social-card.jpg',
      metadata: [
        {name: 'keywords', content: 'neo, blockchain, rust, sdk, neo3, cryptocurrency, smart-contracts, defi'},
        {name: 'description', content: 'NeoRust v0.4.3 - A production-ready Rust SDK for Neo N3 blockchain development. Build high-performance dApps with type-safe, modern Rust. Optimized and production-ready.'},
        {property: 'og:image', content: 'https://neorust.netlify.app/img/neorust-social-card.jpg'},
        {property: 'og:type', content: 'website'},
        {name: 'twitter:card', content: 'summary_large_image'},
        {name: 'twitter:image', content: 'https://neorust.netlify.app/img/neorust-social-card.jpg'},
      ],
      navbar: {
        title: 'NeoRust',
        logo: {
          alt: 'NeoRust Logo',
          src: 'img/logo-icon.svg',
          srcDark: 'img/logo-icon.svg',
          width: 32,
          height: 32,
        },
        items: [
          {
            type: 'docSidebar',
            sidebarId: 'tutorialSidebar',
            position: 'left',
            label: '📚 Documentation',
          },
          {
            to: '/examples',
            label: '💡 Examples',
            position: 'left',
          },
          {
            type: 'dropdown',
            label: '🛠️ Tools',
            position: 'left',
            items: [
              {
                label: '🦀 Rust SDK',
                to: '/sdk/intro',
              },
              {
                label: '🖥️ Desktop GUI',
                to: '/gui/intro',
              },
              {
                label: '⌨️ CLI Tools',
                to: '/cli/intro',
              },
            ],
          },
          {
            type: 'dropdown',
            label: '🔗 Resources',
            position: 'right',
            items: [
              {
                label: '📖 API Reference',
                href: 'https://docs.rs/neo3',
              },
              {
                label: '📦 Crates.io',
                href: 'https://crates.io/crates/neo3',
              },
              {
                label: '🌐 Neo Developer Portal',
                href: 'https://developers.neo.org/',
              },
              {
                label: '🔗 Neo X Documentation',
                href: 'https://docs.neox.network/',
              },
              {
                label: '⭐ GitHub Repository',
                href: 'https://github.com/R3E-Network/NeoRust',
                className: 'dropdown-divider-top',
              },
            ],
          },
        ],
      },
      footer: {
        style: 'dark',
        links: [
          {
            title: '📚 Documentation',
            items: [
              {
                label: 'Getting Started',
                to: '/docs/intro',
              },
              {
                label: 'SDK Documentation',
                to: '/sdk/intro',
              },
              {
                label: 'GUI Documentation',
                to: '/gui/intro',
              },
              {
                label: 'CLI Documentation',
                to: '/cli/intro',
              },
              {
                label: 'Examples',
                to: '/examples',
              },
            ],
          },
          {
            title: '🛠️ Tools',
            items: [
              {
                label: 'Rust SDK',
                to: '/sdk/installation',
              },
              {
                label: 'Desktop GUI',
                to: '/gui/installation',
              },
              {
                label: 'CLI Tools',
                to: '/cli/intro',
              },
            ],
          },
          {
            title: '🌐 Community',
            items: [
              {
                label: 'GitHub',
                href: 'https://github.com/R3E-Network/NeoRust',
              },
              {
                label: 'Discord',
                href: 'https://discord.gg/neo-smart-contracts',
              },
              {
                label: 'Stack Overflow',
                href: 'https://stackoverflow.com/questions/tagged/neo3',
              },
              {
                label: 'Reddit',
                href: 'https://reddit.com/r/NEO',
              },
            ],
          },
          {
            title: '🔗 More',
            items: [
              {
                label: 'API Reference',
                href: 'https://docs.rs/neo3',
              },
              {
                label: 'Crates.io',
                href: 'https://crates.io/crates/neo3',
              },
              {
                label: 'Neo Developer Portal',
                href: 'https://developers.neo.org/',
              },
              {
                label: 'Neo X Documentation',
                href: 'https://docs.neox.network/',
              },
            ],
          },
        ],
        logo: {
          alt: 'NeoRust Logo',
          src: 'img/logo.svg',
          width: 160,
          height: 51,
        },
        copyright: `
          <div style="margin-top: 16px; padding-top: 16px; border-top: 1px solid #333;">
            <p>Copyright © ${new Date().getFullYear()} R3E Network. Built with ❤️ and Docusaurus.</p>
            <p>NeoRust v0.4.3 - Production-Ready Neo N3 Development Suite - Optimized and Enhanced</p>
          </div>
        `,
      },
      prism: {
        theme: lightCodeTheme,
        darkTheme: darkCodeTheme,
        additionalLanguages: ['rust', 'toml', 'bash', 'json', 'yaml', 'typescript', 'javascript'],
        magicComments: [
          {
            className: 'theme-code-block-highlighted-line',
            line: 'highlight-next-line',
            block: {start: 'highlight-start', end: 'highlight-end'},
          },
          {
            className: 'code-block-error-line',
            line: 'This will error',
          },
        ],
      },
      colorMode: {
        defaultMode: 'dark',
        disableSwitch: false,
        respectPrefersColorScheme: true,
      },
      announcementBar: {
        id: 'v0.4.3-release',
        content:
          '🎉 <strong>NeoRust v0.4.3</strong> is now available! Enhanced performance, optimized code quality, and improved reliability. <a target="_blank" rel="noopener noreferrer" href="https://github.com/R3E-Network/NeoRust/releases/tag/v0.4.3">See what\'s new</a>',
        backgroundColor: '#059669',
        textColor: '#ffffff',
        isCloseable: true,
      },
      algolia: {
        // The application ID provided by Algolia
        appId: 'BH4D9OD16A',
        // Public API key: it is safe to commit it
        apiKey: 'eeb9df8bb56a72c7c37527b60b8cb52c',
        indexName: 'neorust',
        // Optional: see doc section below
        contextualSearch: true,
        // Optional: Specify domains where the navigation should occur through window.location instead on history.push
        externalUrlRegex: 'external\\.com|domain\\.com',
        // Optional: Replace parts of the item URLs from Algolia. Useful when using the same search index for multiple deployments using a different baseUrl. You can use regexp or string in the `from` param. For example: localhost:3000 vs myCompany.com/docs
        replaceSearchResultPathname: {
          from: '/docs/', // or as RegExp: /\/docs\//
          to: '/',
        },
        // Optional: Algolia search parameters
        searchParameters: {},
        // Optional: path for search page that enabled by default (`false` to disable it)
        searchPagePath: 'search',
      },
      docs: {
        sidebar: {
          hideable: true,
          autoCollapseCategories: true,
        },
      },
    }),

  plugins: [
    [
      '@docusaurus/plugin-content-docs',
      {
        id: 'sdk',
        path: 'sdk',
        routeBasePath: 'sdk',
        sidebarPath: require.resolve('./sidebars-sdk.js'),
        editUrl: 'https://github.com/R3E-Network/NeoRust/tree/main/website/',
        showLastUpdateAuthor: true,
        showLastUpdateTime: true,
      },
    ],
    [
      '@docusaurus/plugin-content-docs',
      {
        id: 'gui',
        path: 'gui',
        routeBasePath: 'gui',
        sidebarPath: require.resolve('./sidebars-gui.js'),
        editUrl: 'https://github.com/R3E-Network/NeoRust/tree/main/website/',
        showLastUpdateAuthor: true,
        showLastUpdateTime: true,
      },
    ],
    [
      '@docusaurus/plugin-content-docs',
      {
        id: 'cli',
        path: 'cli',
        routeBasePath: 'cli',
        sidebarPath: require.resolve('./sidebars-cli.js'),
        editUrl: 'https://github.com/R3E-Network/NeoRust/tree/main/website/',
        showLastUpdateAuthor: true,
        showLastUpdateTime: true,
      },
    ],
  ],

  headTags: [
    {
      tagName: 'link',
      attributes: {
        rel: 'preconnect',
        href: 'https://fonts.googleapis.com',
      },
    },
    {
      tagName: 'link',
      attributes: {
        rel: 'preconnect',
        href: 'https://fonts.gstatic.com',
        crossorigin: 'anonymous',
      },
    },
    {
      tagName: 'link',
      attributes: {
        rel: 'stylesheet',
        href: 'https://fonts.googleapis.com/css2?family=Inter:wght@300;400;500;600;700;800&family=JetBrains+Mono:wght@300;400;500;600;700&display=swap',
      },
    },
  ],
};

module.exports = config; 