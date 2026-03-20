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
        { to: '/docs/guide/getting-started', label: 'Getting started', position: 'left' },
        { to: '/docs/guide/concepts', label: 'Concepts', position: 'left' },
        { to: '/docs/generated/site-spec', label: 'Spec reference', position: 'left' },
        { to: '/docs/generated/syu-report', label: 'Validation report', position: 'left' },
        { href: 'https://github.com/ugoite/syu', label: 'GitHub', position: 'right' }
      ]
    },
    footer: {
      style: 'dark',
      links: [
        {
          title: 'Guides',
          items: [
            { label: 'Getting started', to: '/docs/guide/getting-started' },
            { label: 'Concepts', to: '/docs/guide/concepts' },
            { label: 'Configuration', to: '/docs/guide/configuration' }
          ]
        },
        {
          title: 'Repository workflow',
          items: [
            { label: 'Validation report', to: '/docs/generated/syu-report' },
            { label: 'Contributing', href: 'https://github.com/ugoite/syu/blob/main/CONTRIBUTING.md' }
          ]
        },
        {
          title: 'Self-hosted spec',
          items: [
            { label: 'Reference index', to: '/docs/generated/site-spec' },
            { label: 'Docs feature', to: '/docs/generated/site-spec/features/docs' }
          ]
        }
      ]
    }
  }
};

module.exports = config;
