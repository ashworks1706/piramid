import { CliLogo } from "../components/CliLogo";

export default function Home() {
  return (
    <main className="landing">
      <div className="landing-inner">
        <CliLogo />
        <p className="landing-tagline">INFERENCE RUNTIME FOR RETRIEVAL SYSTEMS</p>
        <p className="landing-copy">
          Piramid is an inference engine for retrieval systems on one GPU. It
          holds the documents, the model weights and the KV cache in a single
          process, so retrieval can run during generation instead of once
          before it.
        </p>
        <p className="landing-status">
          In active research. Results and write-ups will be published here.
        </p>
      </div>
    </main>
  );
}
