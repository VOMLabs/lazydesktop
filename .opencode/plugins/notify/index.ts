import { Plugin } from "@opencode/plugin"
import { spawn } from "node:child_process"

// 🔧 CONFIGURATION: Set to true to enable this plugin
const ENABLED = false

/** Best-effort desktop notification that a session finished. */
function notifyDone(): void {
  try {
    if (process.platform === "darwin") {
      spawn("osascript", ["-e", 'display notification "Your code is done!" with title "OpenCode"'])
    } else if (process.platform === "linux") {
      spawn("notify-send", ["OpenCode", "Your code is done!"])
    } else {
      console.log("Session idle: your code is done!")
    }
  } catch (error) {
    console.warn("notify plugin: could not send notification", error)
  }
}

/**
 * Notify plugin for OpenCode V2.
 *
 * @remarks
 * Ported from the V1 `.opencode/plugin/notify.ts`. V1 plugins export a
 * function returning hooks; V2 plugins default-export `Plugin.define` and
 * subscribe to events via `ctx.event.subscribe`.
 */
export default Plugin.define({
  id: "notify",
  setup(ctx) {
    if (!ENABLED) {
      console.log("notify plugin is disabled (set ENABLED = true in plugins/notify.ts)")
      return
    }

    const controller = new AbortController()
    void (async () => {
      for await (const event of ctx.event.subscribe({ signal: controller.signal })) {
        if (event.type === "session.idle") {
          notifyDone()
        }
      }
    })()

    return () => controller.abort()
  },
})