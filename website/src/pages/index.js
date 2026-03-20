// FEAT-DOCS-002

import Layout from '@theme/Layout';
import Link from '@docusaurus/Link';

export default function Home() {
  return (
    <Layout title="syu documentation">
      <main style={{ padding: '4rem 1.5rem', maxWidth: '960px', margin: '0 auto' }}>
        <h1>syu documentation</h1>
        <p>
          Explore the guides, the self-hosted specification, and the generated
          artifacts that keep `syu` honest.
        </p>
        <ul>
          <li>
            <Link to="/docs/guide/getting-started">Get started with the CLI</Link>
          </li>
          <li>
            <Link to="/docs/guide/configuration">Configure validation and runtimes</Link>
          </li>
          <li>
            <Link to="/docs/generated/site-spec">Browse the self-hosted specification</Link>
          </li>
        </ul>
      </main>
    </Layout>
  );
}
