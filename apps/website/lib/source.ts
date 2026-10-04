import { loader } from "fumadocs-core/source";
import { defineDocs } from "fumadocs-mdx/macro";

const docs = defineDocs({ dir: "content/docs" });

/** The user docs under content/docs, served at /docs. */
export const source = loader({
  baseUrl: "/docs",
  source: docs.toFumadocsSource(),
});
