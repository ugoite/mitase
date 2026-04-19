// FEAT-DOCS-002

import Layout from '@theme/Layout';
import Link from '@docusaurus/Link';

const layers = [
  {
    title: 'Philosophy',
    description: 'Capture the stable ideals and trade-offs that should survive implementation changes.',
    to: '/docs/guide/concepts#philosophy'
  },
  {
    title: 'Policy',
    description: 'Turn those ideals into repository-wide rules that contributors can follow consistently.',
    to: '/docs/guide/concepts#policy'
  },
  {
    title: 'Requirements',
    description: 'Define concrete obligations that can be verified through tests and repository evidence.',
    to: '/docs/guide/concepts#requirements'
  },
  {
    title: 'Features',
    description: 'Connect implemented behavior back to requirements and forward to the code that proves it exists.',
    to: '/docs/guide/concepts#features'
  }
];

const journeys = [
  {
    title: 'Decide repository fit',
    description: 'Read the repository-fit guide before installing when you are still deciding whether syu is the right adoption step.',
    to: '/docs/guide/getting-started#is-syu-right-for-this-repository'
  },
  {
    title: 'Avoid spec anti-patterns',
    description:
      'Learn the common bad-but-valid four-layer shapes before a green spec turns into a painful rewrite.',
    to: '/docs/guide/spec-antipatterns'
  },
  {
    title: 'Adopt an existing repository',
    description:
      'Phase syu into a repo that already has code, tests, and docs instead of translating the greenfield flow by hand.',
    to: '/docs/guide/existing-repository'
  },
  {
    title: 'Start a workspace',
    description: 'Scaffold a project, fill in the layered spec, and run validate without guessing the layout.',
    to: '/docs/guide/getting-started'
  },
  {
    title: 'Explore the browser UI',
    description:
      'Take the visual-first path through `syu app` when you want tabs, trace links, and validation context in one browser view.',
    to: '/docs/guide/app'
  },
  {
    title: 'Follow a full tutorial',
    description: 'Build a realistic four-layer example from scratch when you want the full repository story.',
    to: '/docs/guide/tutorial'
  },
  {
    title: 'Troubleshoot a broken workspace',
    description: 'Jump straight to the common validation, traceability, and workflow failure patterns.',
    to: '/docs/guide/troubleshooting'
  },
  {
    title: 'Upgrade an existing workspace',
    description:
      'Jump straight to the migration guide when a new alpha release changes config, validation defaults, or guide structure.',
    to: '/docs/guide/migration'
  },
  {
    title: 'Stay in VS Code',
    description: 'Run the checked-in editor extension so diagnostics and trace links stay inside your editor.',
    to: '/docs/guide/vscode-extension'
  },
  {
    title: 'Tune validation',
    description: 'Review config switches for autofix, planned work, orphan checks, and runtime behavior.',
    to: '/docs/guide/configuration'
  },
  {
    title: 'Inspect the self-hosted spec',
    description: 'Browse the generated reference pages that explain how this repository uses syu on itself.',
    to: '/docs/generated/site-spec'
  },
  {
    title: 'Check the latest report',
    description: 'Read the checked-in validation report to see the repository state without running the CLI first.',
    to: '/docs/generated/syu-report'
  }
];

export default function Home() {
  return (
    <Layout
      title="syu documentation"
      description="Browse the four-layer model, contributor workflows, and the self-hosted syu specification."
    >
      <header className="hero hero--primary siteHero">
        <div className="container">
          <p className="siteHeroEyebrow">Specification-driven development for real repositories</p>
          <h1 className="siteHeroTitle">Keep the spec close to the repository</h1>
          <p className="siteHeroLead">
            Decide whether repository-native traceability fits your repo, then browse
            the four specification layers, common contributor journeys, and self-hosted
            validation output in one place.
          </p>
          <div className="siteHeroActions">
            <Link className="button button--secondary button--lg" to="/docs/guide/getting-started">
              Get started
            </Link>
            <Link
              className="button button--secondary button--lg"
              to="https://github.com/ugoite/syu#quick-start"
            >
              Quick start on GitHub
            </Link>
            <Link
              className="button button--outline button--lg siteHeroOutlineButton"
              to="/docs/generated/site-spec"
            >
              Browse the self-hosted spec
            </Link>
          </div>
        </div>
      </header>

      <main>
        <section className="siteSection">
          <div className="container">
            <div className="siteSectionHeader">
              <h2>Four specification layers</h2>
              <p>
                <code>syu</code> keeps philosophy, policy, requirements, and features separate
                so the repository can explain itself from intent down to code and tests.
              </p>
            </div>
            <div className="siteCardGrid">
              {layers.map((layer) => (
                <article className="siteCard" key={layer.title}>
                  <h3>{layer.title}</h3>
                  <p>{layer.description}</p>
                  <Link className="siteCardLink" to={layer.to}>
                    {`Open the ${layer.title} layer`}
                  </Link>
                </article>
              ))}
            </div>
          </div>
        </section>

        <section className="siteSection siteSectionAlt">
          <div className="container">
            <div className="siteSectionHeader">
              <h2>Common journeys</h2>
              <p>
                Start from the task you are trying to complete and jump directly to
                the most relevant guide, reference page, or generated artifact.
              </p>
            </div>
            <div className="siteCardGrid">
              {journeys.map((journey) => (
                <article className="siteCard" key={journey.title}>
                  <h3>{journey.title}</h3>
                  <p>{journey.description}</p>
                  <Link className="siteCardLink" to={journey.to}>
                    {`Follow the ${journey.title} journey`}
                  </Link>
                </article>
              ))}
            </div>
          </div>
        </section>

        <section className="siteSection">
          <div className="container siteCallout">
            <div>
              <h2>Stay close to checked-in source</h2>
              <p>
                The site renders the checked-in documentation tree directly, so guides,
                generated specification pages, and the latest validation report stay
                aligned with the repository state instead of drifting into a separate
                content source.
              </p>
            </div>
            <Link className="button button--primary button--lg" to="/docs/guide/configuration">
              Review configuration
            </Link>
          </div>
        </section>
      </main>
    </Layout>
  );
}
