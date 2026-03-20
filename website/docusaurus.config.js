// FEAT-DOCS-002

/** @type {import('@docusaurus/types').Config} */
const config = {
  title: 'syu',
  tagline: 'Specification-driven development that stays close to the repository',
  favicon: 'img/favicon.svg',
  url: 'https://ugoite.github.io',
  baseUrl: '/syu/',
  organizationName: 'ugoite',
  projectName: 'syu',
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
      title: 'syu',
      items: [
        { to: '/docs/guide/getting-started', label: 'Docs', position: 'left' },
        { to: '/docs/generated/site-spec', label: 'Spec', position: 'left' },
        { href: 'https://github.com/ugoite/syu', label: 'GitHub', position: 'right' }
      ]
    },
    footer: {
      style: 'dark',
      links: [
        {
          title: 'Docs',
          items: [
            { label: 'Getting started', to: '/docs/guide/getting-started' },
            { label: 'Configuration', to: '/docs/guide/configuration' }
          ]
        },
        {
          title: 'Specification',
          items: [
            { label: 'Reference index', to: '/docs/generated/site-spec' },
            { label: 'Validation report', to: '/docs/generated/syu-report' }
          ]
        }
      ]
    }
  }
};

module.exports = config;
