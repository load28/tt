/* --------------------------------------------------------------------------
 * One `ttc --server` session, asked for every module of a build.
 *
 * A `ttc -p` per module opens the whole TypeScript project again for every
 * module, because the compiler refines the storage annotations it emits
 * with the project's types. The server keeps that project open between
 * requests, so a build pays for it once.
 *
 * Requests carry ids and may overlap; the server answers them in order.
 * When the process ends with requests in flight, each is asked once more
 * of a fresh process; a second end fails the request with what the
 * process said on stderr. Nothing falls back to another way of compiling.
 * ----------------------------------------------------------------------- */
import { spawn } from "node:child_process";
import { createInterface } from "node:readline";

/** How much of the server's stderr a crash report quotes. */
const STDERR_TAIL = 16 * 1024;

class ServerEnded extends Error {}

class Session {
  constructor(compiler) {
    this.pending = new Map();
    this.nextId = 0;
    this.stderr = "";
    this.ended = null;
    this.child = spawn(compiler, ["--server"], { stdio: ["pipe", "pipe", "pipe"], windowsHide: true });
    this.child.stdin.on("error", () => {});
    this.child.stderr.setEncoding("utf8");
    this.child.stderr.on("data", (chunk) => {
      this.stderr = (this.stderr + chunk).slice(-STDERR_TAIL);
    });
    createInterface({ input: this.child.stdout, crlfDelay: Infinity }).on("line", (line) => this.answer(line));
    this.child.on("error", (error) => this.end(`cannot run ${compiler}: ${error.message}`));
    this.child.on("close", (code, signal) =>
      this.end(`${compiler} --server exited ${signal === null ? `with code ${code}` : `on ${signal}`}`),
    );
    this.idle();
  }

  request(method, params) {
    if (this.ended !== null) return Promise.reject(new ServerEnded(this.ended));
    const id = ++this.nextId;
    return new Promise((resolve, reject) => {
      this.pending.set(id, { resolve, reject });
      this.busy();
      this.child.stdin.write(`${JSON.stringify({ id, method, params })}\n`);
    });
  }

  answer(line) {
    let response;
    try {
      response = JSON.parse(line);
    } catch {
      this.end(`the compiler server answered with a line that is not JSON: ${line}`);
      this.child.kill();
      return;
    }
    const waiting = this.pending.get(response.id);
    if (waiting === undefined) return;
    this.pending.delete(response.id);
    if (this.pending.size === 0) this.idle();
    if (response.error !== undefined) waiting.reject(new Error(response.error));
    else waiting.resolve(response.result);
  }

  end(reason) {
    if (this.ended !== null) return;
    const said = this.stderr.trim();
    this.ended = said === "" ? reason : `${reason}:\n${said}`;
    for (const { reject } of this.pending.values()) reject(new ServerEnded(this.ended));
    this.pending.clear();
  }

  // An idle session must not keep the bundler's process alive: when that
  // process exits, the server reads the end of its input and exits too.
  busy() {
    this.child.ref();
    this.child.stdout.ref?.();
    this.child.stderr.ref?.();
  }

  idle() {
    this.child.unref();
    this.child.stdout.unref?.();
    this.child.stderr.unref?.();
  }

  close() {
    this.ended ??= "the compiler server was closed";
    this.child.stdin.end();
  }
}

/**
 * The compiler servers of one compiler binary: one per working directory,
 * the directory a `ttc -p` would have run in, started on its first request.
 */
export class CompilerServer {
  constructor(compiler) {
    this.compiler = compiler;
    this.sessions = new Map();
  }

  async request(method, params) {
    const cwd = process.cwd();
    const session = this.current(cwd);
    try {
      return await session.request(method, params);
    } catch (error) {
      if (!(error instanceof ServerEnded)) throw error;
    }
    try {
      return await this.current(cwd).request(method, params);
    } catch (error) {
      if (error instanceof ServerEnded) throw new Error(`the compiler server stopped twice: ${error.message}`);
      throw error;
    }
  }

  current(cwd) {
    let session = this.sessions.get(cwd);
    if (session === undefined || session.ended !== null) {
      session = new Session(this.compiler);
      this.sessions.set(cwd, session);
    }
    return session;
  }

  close() {
    for (const session of this.sessions.values()) session.close();
    this.sessions.clear();
  }
}
