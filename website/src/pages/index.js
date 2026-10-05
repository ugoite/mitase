// FEAT-DOCS-002

import Layout from '@theme/Layout';
import Link from '@docusaurus/Link';

const intentToEvidence = [
  {
    title: 'Principles',
    description: 'Philosophy and Policy preserve durable ideals and repository-wide rules.',
    to: '/docs/understand/model/concepts#philosophy'
  },
  {
    title: 'Obligations',
    description: 'Requirements and Criteria state observable behavior and independently verifiable conditions.',
    to: '/docs/understand/model/concepts#criterion'
  },
  {
    title: 'Implementation',
    description: 'Features and Bindings connect those obligations to exact repository targets.',
    to: '/docs/understand/model/concepts#binding'
  },
  {
    title: 'Verification',
    description: 'Verification Claims identify the exact evidence designated to verify each Criterion.',
    to: '/docs/understand/model/domain-glossary'
  }
];

const chooseYourPath = [
  {
    title: 'Getting started',
    description:
      'Choose this if you are new to mitase, want the guided first-run path, and do not already know the specification graph.',
    to: '/docs/start-here/first-run/getting-started'
  },
  {
    title: 'Quick start',
    description:
      'Stay on the shortest site-local install-to-check path when you want the fastest route into `mitase check .`.',
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
      'Learn the common bad-but-valid specification-graph shapes before a green spec turns into a painful rewrite.',
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
    description: 'Scaffold a project, fill in the connected spec, and run check without guessing the layout.',
    to: '/docs/start-here/first-run/getting-started'
  },
  {
    title: 'Follow a full tutorial',
    description: 'Build a realistic intent-to-evidence example from scratch when you want the full repository story.',
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
    description: 'Review configuration for validation, orphan checks, and runtime behavior.',
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
      description="Browse the intent-to-evidence model, contributor workflows, and the self-hosted mitase specification."
    >
      <header className="hero hero--primary siteHero">
        <div className="container">
          <p className="siteHeroEyebrow">Repository-native executable specifications</p>
          <h1 className="siteHeroTitle">Keep repository promises connected to implementation and verification</h1>
          <p className="siteHeroLead">
            Declare what must be true, bind it to exact implementation and verification
            targets, and let Mitase check that those relationships still resolve as the
            repository changes.
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
              <h2>From intent to evidence</h2>
              <p>
                <code>mitase</code> keeps declared intent, exact implementation targets, and
                verification evidence connected so those relationships can be checked
                mechanically as the repository changes.
              </p>
            </div>
            <div className="siteCardGrid">
              {intentToEvidence.map((item) => (
                <article className="siteCard" key={item.title}>
                  <h3>{item.title}</h3>
                  <p>{item.description}</p>
                  <Link className="siteCardLink" to={item.to}>
                    {`Open ${item.title}`}
                  </Link>
                </article>
              ))}
            </div>
          </div>
        </section>

        <section className="siteSection">
          <div className="container">
            <div className="siteSectionHeader">
              <h2>Why exact relationships?</h2>
              <p>
                Exact relationships reduce rediscovery. Once a Criterion is connected to
                its implementation and verification targets, contributors, CI, IDEs, and
                external agents can inspect the same repository-owned structure instead
                of reconstructing those relationships from memory every time the code
                changes.
              </p>
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
