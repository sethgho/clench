import { createCliRenderer } from "@opentui/core"
import { render } from "@opentui/solid"
import { openSync } from "node:fs"
import { constants } from "node:os"
import { ReadStream } from "node:tty"
import pkg from "../package.json"
import { parseArgs, USAGE } from "./args"
import { ExtractError, extractDocument, isEngineCrash, loadEngine, Session, type Document } from "./engine"
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
    loaded = await load(args.source, { type: args.type })
  } catch (error) {
    if (fetching) process.stderr.write("\n")
    fail(error instanceof LoadError ? error.message : String(error))
  }
  if (fetching) process.stderr.write("\r\x1b[K")

  const where = args.source === "-" ? "standard input" : args.source
  let doc: Document
  try {
    loadEngine()
    doc = extractDocument(loaded.bytes, loaded.contentType, loaded.url || (fetching ? args.source : undefined))
  } catch (error) {
    // The engine instance is not used again after a failure: the process ends here.
    fail(`could not read ${where}: ${error instanceof ExtractError ? error.message : String(error)}`)
  }
  const session = new Session(doc, { wpm: args.wpm })
  if (session.wordCount === 0) fail(`nothing to read in ${where}: no readable text found`)

  const renderer = await createCliRenderer({
    stdin: keyboard(),
    exitOnCtrlC: false,
    exitSignals: [],
    useMouse: false,
    screenMode: "alternate-screen",
  })
  // Termination by signal restores the terminal first and exits 128 + the
  // signal number, as shells report it.
  let status = 0
  let farewell: string | undefined
  for (const name of ["SIGHUP", "SIGINT", "SIGQUIT", "SIGTERM"] as const) {
    process.on(name, () => {
      status = 128 + constants.signals[name]
      renderer.destroy()
    })
  }
  // A trap inside the engine leaves it unusable: leave the terminal and say so.
  process.on("uncaughtException", (error) => {
    status = 1
    farewell = isEngineCrash(error)
      ? "the reading engine crashed; please report the page that caused it"
      : error instanceof Error
        ? (error.stack ?? error.message)
        : String(error)
    renderer.destroy()
  })
  await render(() => <App doc={doc} session={session} theme={makeTheme(colorWanted())} />, renderer)
  await renderer.closed
  if (farewell) process.stderr.write(`meatread: ${farewell}\n`)
  process.exit(status)
}

await main()
