import { CliLogo } from "../components/CliLogo";
import { Console } from "../components/Console";
import { entries, files } from "../lib/console";

export default function Home() {
  return (
    <main className="page">
      <Console entries={entries()} files={files()}>
        <CliLogo />
      </Console>
    </main>
  );
}
