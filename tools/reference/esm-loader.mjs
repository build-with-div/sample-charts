// Node module hooks that load Recharts' browser build, `recharts/es6`, the
// build bundlers pick through the package's "module" field. Its files are ES
// modules in a package without "type": "module", and they import relative
// paths without extensions, as bundlers allow. Through it, Recharts imports
// `victory-vendor/d3-*` with the "import" condition, which resolves to the npm
// d3 packages; the CommonJS build (`recharts/lib`) uses older d3 copies
// vendored inside victory-vendor instead, so only this build matches a page.
//
// Registered by axis_fixtures.mjs with module.register().

export async function resolve(specifier, context, nextResolve) {
  try {
    return await nextResolve(specifier, context);
  } catch (error) {
    const relative = specifier.startsWith("./") || specifier.startsWith("../");
    const missing = error?.code === "ERR_MODULE_NOT_FOUND" || error?.code === "ERR_UNSUPPORTED_DIR_IMPORT";
    if (!missing || !relative) throw error;
    for (const suffix of [".js", "/index.js"]) {
      try {
        return await nextResolve(specifier + suffix, context);
      } catch {
        // try the next suffix
      }
    }
    throw error;
  }
}

export async function load(url, context, nextLoad) {
  if (/[\\/]node_modules[\\/]recharts[\\/]es6[\\/]/.test(url)) {
    const result = await nextLoad(url, { ...context, format: "module" });
    return { ...result, format: "module" };
  }
  return nextLoad(url, context);
}
