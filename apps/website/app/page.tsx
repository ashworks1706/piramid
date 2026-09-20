import Link from "next/link";
import { CliLogo } from "../components/CliLogo";
import { Readme } from "../components/Readme";

export default function Home() {
  return (
    <main className="page">
      <section className="landing">
        <div className="landing-inner">
          <CliLogo />

          <p className="landing-tagline">
            inference runtime for retrieval systems
          </p>

          <div className="landing-actions">
            <code className="landing-install select-all">
              cargo install --git https://github.com/ashworks1706/piramid
              piramid --locked --features inference-candle,gpu-cuda
            </code>
            <a
              href="https://github.com/ashworks1706/piramid"
              className="landing-link"
            >
              github
            </a>
            <Link href="/blogs" className="landing-link">
              blog
            </Link>
          </div>
        </div>
      </section>

      <Readme />
    </main>
  );
}
