import { DocsLayout } from "fumadocs-ui/layouts/docs";
import { RootProvider } from "fumadocs-ui/provider/next";
import { source } from "@/lib/source";

export default function Layout({ children }: LayoutProps<"/docs">) {
  return (
    <RootProvider theme={{ forcedTheme: "dark" }}>
      <DocsLayout
        tree={source.getPageTree()}
        nav={{ title: "PIRAMID" }}
        themeSwitch={{ enabled: false }}
      >
        {children}
      </DocsLayout>
    </RootProvider>
  );
}
