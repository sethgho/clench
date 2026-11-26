import { createCliRenderer } from "@opentui/core"
import { render } from "@opentui/solid"
import { openSync } from "node:fs"
import { ReadStream } from "node:tty"
import pkg from "../package.json"
import { parseArgs, USAGE } from "./args"
import { extract, isEngineCrash, loadEngine, Session, type Document } from "./engine"
import { isUrl, load, LoadError } from "./source"
import { colorWanted, makeTheme } from "./theme"
import { App } from "./ui/app"

function fail(message: string, code = 1): never {
  process.stderr.write(`meatread: ${message}\n`)
  process.exit(code)
}

/** Keyboard input: standard input when it is a terminal, else the controlling terminal. */
function keyboard(): NodeJS.ReadStream | undefined {
  if (process.stdin.isTTY) return undefined
  try {
    return new ReadStream(openSync("/dev/tty", "r")) as unknown as NodeJS.ReadStream
  } catch {
    fail("no terminal to read keys from")
  }
}

async function main(): Promise<void> {
  const args = parseArgs(Bun.argv.slice(2))
  if (args.kind === "help") {
    process.stdout.write(USAGE + "\n")
    return
  }
  if (args.kind === "version") {
    process.stdout.write(`meatread ${pkg.version}\n`)
    return
  }
  if (args.kind === "error") fail(`${args.message}\n\n${USAGE}`, 2)
  if (!process.stdout.isTTY) fail("meatread shows text one word at a time and needs a terminal to draw on")

  const fetching = isUrl(args.source)
  if (fetching) process.stderr.write(`fetching ${args.source} …`)
  let loaded
  try {
    loaded = await load(args.source)
  } catch (error) {
    if (fetching) process.stderr.write("\n")
    fail(error instanceof LoadError ? error.message : String(error))
  }
  if (fetching) process.stderr.write("\r\x1b[K")

  const where = args.source === "-" ? "standard input" : args.source
  let doc: Document
  try {
    loadEngine()
    doc = extract(loaded.bytes, loaded.contentType, loaded.url || (fetching ? args.source : undefined))
  } catch (error) {
    if (isEngineCrash(error)) fail(`could not read ${where}: the extractor failed on this document`)
    fail(`could not read ${where}: ${error instanceof Error ? error.message : String(error)}`)
  }
  const session = new Session(doc, { wpm: args.wpm })
  if (session.wordCount === 0) fail(`nothing to read in ${where}: no readable text found`)

  const renderer = await createCliRenderer({
    stdin: keyboard(),
    exitOnCtrlC: false,
    useMouse: false,
    screenMode: "alternate-screen",
  })
  await render(() => <App doc={doc} session={session} theme={makeTheme(colorWanted())} />, renderer)
  await renderer.closed
  process.exit(0)
}

await main()
