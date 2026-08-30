// FEAT-DOCS-002

/** @type {import('@docusaurus/types').Config} */
const config = {
  title: 'mitase',
  tagline: 'Specification-driven development that stays close to the repository',
  favicon: 'img/favicon.svg',
  url: 'https://ugoite.github.io',
  baseUrl: '/mitase/',
  organizationName: 'ugoite',
  projectName: 'mitase',
  onBrokenLinks: 'throw',
  markdown: {
    hooks: {
      onBrokenMarkdownLinks: 'throw'
    }
  },
  i18n: {
    defaultLocale: 'en',
    locales: ['en']
  },
  presets: [
    [
      'classic',
      {
        docs: {
          path: '../docs',
          routeBasePath: 'docs',
          sidebarPath: require.resolve('./sidebars.js')
        },
        blog: false,
        pages: {},
        theme: {
          customCss: require.resolve('./src/css/custom.css')
        }
      }
    ]
  ],
  themeConfig: {
    navbar: {
      title: 'mitase',
      items: [
        { to: '/docs/start-here', label: 'Start here', position: 'left' },
        { to: '/docs/understand', label: 'Understand', position: 'left' },
        { to: '/docs/reference/specification', label: 'Specification reference', position: 'left' },
        { to: '/docs/reference/status', label: 'Repository status', position: 'left' },
        { href: 'https://github.com/ugoite/mitase', label: 'GitHub', position: 'right' }
      ]
    },
    docs: {
      sidebar: {
        autoCollapseCategories: true,
        hideable: true
      }
    },
    footer: {
      style: 'dark',
      links: [
        {
          title: 'Start here',
          items: [
            { label: 'Getting started', to: '/docs/start-here/first-run/getting-started' },
            { label: 'Tutorial', to: '/docs/start-here/first-run/tutorial' },
            { label: 'Adopt an existing repository', to: '/docs/start-here/adopt' }
          ]
        },
        {
          title: 'Workflows',
          items: [
            { label: 'Configuration', to: '/docs/workflows/repository/configuration' },
            { label: 'Contribute', to: '/docs/contribute' }
          ]
        },
        {
          title: 'Reference',
          items: [
            { label: 'Specification reference', to: '/docs/reference/specification' },
            { label: 'Validation report', to: '/docs/reference/status/validation-report' },
            { label: 'Contributing', href: 'https://github.com/ugoite/mitase/blob/main/CONTRIBUTING.md' }
          ]
        }
      ]
    }
  }
};

module.exports = config;
