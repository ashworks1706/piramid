import { CliLogo } from "../components/CliLogo";
import { Console } from "../components/Console";
import { Readme } from "../components/Readme";
import { entries } from "../lib/console";

export default function Home() {
  return (
    <main className="page">
      <Console entries={entries()}>
        <CliLogo />
      </Console>

      <Readme />
    </main>
  );
}
