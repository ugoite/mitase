// FEAT-DOCS-002

import Layout from '@theme/Layout';
import Link from '@docusaurus/Link';

const layers = [
  {
    title: 'Philosophy',
    description: 'Capture the stable ideals and trade-offs that should survive implementation changes.',
    to: '/docs/understand/model/concepts#philosophy'
  },
  {
    title: 'Policy',
    description: 'Turn those ideals into repository-wide rules that contributors can follow consistently.',
    to: '/docs/understand/model/concepts#policy'
  },
  {
    title: 'Requirements',
    description: 'Define concrete obligations that can be verified through tests and repository evidence.',
    to: '/docs/understand/model/concepts#requirements'
  },
  {
    title: 'Features',
    description: 'Connect implemented behavior back to requirements and forward to the code that proves it exists.',
    to: '/docs/understand/model/concepts#features'
  }
];

const chooseYourPath = [
  {
    title: 'Getting started',
    description:
      'Choose this if you are new to mitase, want the guided first-run path, and do not already know the four-layer model.',
    to: '/docs/start-here/first-run/getting-started'
  },
  {
    title: 'Quick start',
    description:
      'Stay on the shortest site-local install-to-validate path when you want the fastest route into `mitase validate workspace .`.',
    to: '/docs/start-here/first-run/getting-started#quick-start-commands'
  },
  {
    title: 'Tutorial',
    description:
      'Follow the longer repository story when you want more narrative context than a first-run checklist.',
    to: '/docs/start-here/first-run/tutorial'
  },
  {
    title: 'Editor-first path',
    description:
      'Open the VS Code extension guide when you want diagnostics, spec navigation, and trace lookups before you memorize the CLI.',
    to: '/docs/workflows/integrations/vscode-extension'
  },
  {
    title: 'Migration / upgrade',
    description:
      'Use the release-specific upgrade steps when you already have a mitase workspace and need to move between alpha versions safely.',
    to: '/docs/workflows/repository/migration'
  },
  {
    title: 'Visual explorer',
    description:
      'Compare template-backed and example-backed paths when you want to see the main shapes before choosing one.',
    to: '/docs/start-here/adopt/examples-and-templates'
  },
  {
    title: 'Reviewer workflow',
    description:
      'Open the review loop guide when a pull request already exists and you need one concrete path through spec IDs, traced code, and git history.',
    to: '/docs/contribute/reviewing/reviewer-workflow'
  },
  {
    title: 'Trace adapter matrix',
    description:
      'Check which built-in languages support symbol validation only versus richer `doc_contains` and strict coverage checks.',
    to: '/docs/workflows/integrations/trace-adapter-support'
  },
  {
    title: 'Troubleshooting',
    description:
      'Jump straight to validation and traceability repair guidance when an existing workspace is already blocked.',
    to: '/docs/workflows/repository/troubleshooting'
  }
];

const journeys = [
  {
    title: 'Decide repository fit',
    description: 'Read the repository-fit guide before installing when you are still deciding whether mitase is the right adoption step.',
    to: '/docs/start-here/first-run/getting-started#is-mitase-right-for-this-repository'
  },
  {
    title: 'Avoid spec anti-patterns',
    description:
      'Learn the common bad-but-valid four-layer shapes before a green spec turns into a painful rewrite.',
    to: '/docs/understand/quality/spec-antipatterns'
  },
  {
    title: 'Adopt an existing repository',
    description:
      'Phase mitase into a repo that already has code, tests, and docs instead of translating the greenfield flow by hand.',
    to: '/docs/start-here/adopt/existing-repository'
  },
  {
    title: 'Start a workspace',
    description: 'Scaffold a project, fill in the layered spec, and run validate without guessing the layout.',
    to: '/docs/start-here/first-run/getting-started'
  },
  {
    title: 'Keep a command card open',
    description:
      'Use one compact docs-site page for the core install, init, validate, browse, and reviewer commands.',
    to: '/docs/workflows/work/command-card'
  },
  {
    title: 'Follow a full tutorial',
    description: 'Build a realistic four-layer example from scratch when you want the full repository story.',
    to: '/docs/start-here/first-run/tutorial'
  },
  {
    title: 'Troubleshoot a broken workspace',
    description: 'Jump straight to the common validation, traceability, and workflow failure patterns.',
    to: '/docs/workflows/repository/troubleshooting'
  },
  {
    title: 'Upgrade an existing workspace',
    description:
      'Jump straight to the migration guide when a new alpha release changes config, validation defaults, or guide structure.',
    to: '/docs/workflows/repository/migration'
  },
  {
    title: 'Stay in VS Code',
    description: 'Run the checked-in editor extension so diagnostics and trace links stay inside your editor.',
    to: '/docs/workflows/integrations/vscode-extension'
  },
  {
    title: 'Tune validation',
    description: 'Review config switches for autofix, planned work, orphan checks, and runtime behavior.',
    to: '/docs/workflows/repository/configuration'
  },
  {
    title: 'Understand trace adapter support',
    description:
      'Compare which languages support rich inspection, which stay pattern-based, and where `doc_contains` is still unavailable.',
    to: '/docs/workflows/integrations/trace-adapter-support'
  },
  {
    title: 'Inspect the self-hosted spec',
    description: 'Browse the generated reference pages that explain how this repository uses mitase on itself.',
    to: '/docs/reference/specification'
  },
  {
    title: 'Check the latest report',
    description: 'Read the checked-in validation report to see the repository state without running the CLI first.',
    to: '/docs/reference/status/validation-report'
  }
];

export default function Home() {
  return (
    <Layout
      title="mitase documentation"
      description="Browse the four-layer model, contributor workflows, and the self-hosted mitase specification."
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
            <Link className="button button--secondary button--lg" to="/docs/start-here/first-run/getting-started">
              Get started
            </Link>
            <Link className="button button--secondary button--lg" to="/docs/start-here/first-run/tutorial">
              Follow the tutorial
            </Link>
            <Link
              className="button button--outline button--lg siteHeroOutlineButton"
              to="/docs/contribute/reviewing/reviewer-workflow"
            >
              Reviewer workflow
            </Link>
            <Link
              className="button button--outline button--lg siteHeroOutlineButton"
              to="/docs/workflows/repository/troubleshooting"
            >
              Troubleshoot a workspace
            </Link>
          </div>
        </div>
      </header>

      <main>
        <section className="siteSection">
          <div className="container">
            <div className="siteSectionHeader">
              <h2>Choose your path</h2>
              <p>
                Stay inside the published docs and start from the same task-oriented entry
                points the checked-in README uses.
              </p>
            </div>
            <div className="siteCardGrid">
              {chooseYourPath.map((path) => (
                <article className="siteCard" key={path.title}>
                  <h3>{path.title}</h3>
                  <p>{path.description}</p>
                  <Link className="siteCardLink" to={path.to}>
                    {`Open the ${path.title} path`}
                  </Link>
                </article>
              ))}
            </div>
          </div>
        </section>

        <section className="siteSection siteSectionAlt">
          <div className="container">
            <div className="siteSectionHeader">
              <h2>Four specification layers</h2>
              <p>
                <code>mitase</code> keeps philosophy, policy, requirements, and features separate
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

        <section className="siteSection">
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

        <section className="siteSection siteSectionAlt">
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
            <Link className="button button--primary button--lg" to="/docs/workflows/repository/configuration">
              Review configuration
            </Link>
          </div>
        </section>
      </main>
    </Layout>
  );
}
