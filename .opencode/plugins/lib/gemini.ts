import { access, mkdir, readFile, stat, writeFile } from "fs/promises"
import { basename, extname, join, resolve } from "path"
import { getApiKey } from "./env"

export interface ImageConfig {
  outputDir?: string
  useTimestamp?: boolean
  preserveOriginal?: boolean
  customName?: string
}

/**
 * Gemini image generation / editing / analysis backend.
 *
 * @remarks
 * Ported from the V1 `.opencode/tool/gemini` tool. Bun-specific APIs
 * (`Bun.file`, `Bun.write`) were replaced with portable `node:fs` calls so
 * the plugin runs in OpenCode V2's plugin runtime.
 */

const GENERATE_ENDPOINT = "https://generativelanguage.googleapis.com/v1beta/models/gemini-2.5-flash-image-preview:generateContent"
const ANALYZE_ENDPOINT = "https://generativelanguage.googleapis.com/v1beta/models/gemini-1.5-flash:generateContent"

function isTestMode(): boolean {
  return process.env.GEMINI_TEST_MODE === "true"
}

async function getGeminiApiKey(): Promise<string> {
  if (isTestMode()) return "test-api-key"
  return getApiKey("GEMINI_API_KEY")
}

async function fileExists(p: string): Promise<boolean> {
  try {
    await access(p)
    return true
  } catch {
    return false
  }
}

/** Accepts a file path ("./img.png") or a data URL ("data:image/png;base64,..."). */
async function parseImageInput(input: string): Promise<{ mime: string; base64: string }> {
  if (input.startsWith("data:")) {
    const base64 = input.split(",")[1]
    const mime = input.substring(5, input.indexOf(";"))
    return { mime, base64 }
  }

  const data = await readFile(resolve(input))
  return { mime: "image/png", base64: Buffer.from(data).toString("base64") }
}

async function ensureDirectoryExists(dirPath: string): Promise<void> {
  try {
    await mkdir(dirPath, { recursive: true })
  } catch {
    // Directory might already exist, that's fine.
  }
}

function getDateBasedPath(baseDir?: string): string {
  // Default to assets/images at the repository root.
  if (!baseDir) {
    baseDir = resolve(process.cwd(), "assets/images")
  }
  const today = new Date().toISOString().split("T")[0] // YYYY-MM-DD
  return join(baseDir, today)
}

async function getUniqueFilename(
  directory: string,
  baseName: string,
  extension: string,
  isEdit = false,
): Promise<string> {
  await ensureDirectoryExists(directory)

  if (!isEdit) {
    const baseFilename = join(directory, `${baseName}${extension}`)
    if (!(await fileExists(baseFilename))) return baseFilename

    // Add a timestamp if the file already exists.
    const timestamp = new Date().toISOString().replace(/[:.]/g, "-").slice(0, -5)
    return join(directory, `${baseName}_${timestamp}${extension}`)
  }

  // For edits, use incremental numbering.
  let counter = 1
  let filename: string
  do {
    const editSuffix = `_edit_${counter.toString().padStart(3, "0")}`
    filename = join(directory, `${baseName}${editSuffix}${extension}`)
    counter++
  } while (await fileExists(filename))

  return filename
}

function stripImageExtension(name: string): string {
  if (name.endsWith(".png") || name.endsWith(".jpg") || name.endsWith(".jpeg")) {
    return name.substring(0, name.lastIndexOf("."))
  }
  return name
}

async function saveB64(base64: string, outputPath: string): Promise<number> {
  await writeFile(outputPath, Buffer.from(base64, "base64"))
  if (!(await fileExists(outputPath))) throw new Error(`Failed to save file to ${outputPath}`)
  return (await stat(outputPath)).size
}

/** Extract inline image data from a Gemini generateContent response. */
function extractContentB64(json: Record<string, unknown>): string | null {
  const candidates = json?.candidates as Array<{ content?: { parts?: Array<{ inlineData?: { data?: string } }> } }> | undefined
  if (!candidates || candidates.length === 0) throw new Error("No candidates in response")

  const parts = candidates[0]?.content?.parts
  if (!parts || parts.length === 0) throw new Error("No parts in response")

  for (const part of parts) {
    if (part.inlineData?.data) return part.inlineData.data
  }
  return null
}

export async function generateImage(prompt: string, config: ImageConfig = {}): Promise<string> {
  const apiKey = await getGeminiApiKey()

  if (isTestMode()) {
    const generationsDir = join(config.outputDir || getDateBasedPath(), "generations")
    const outputPath = await getUniqueFilename(generationsDir, stripImageExtension(config.customName || "generated"), ".png", false)
    return `[TEST MODE] Would generate image: ${outputPath} for prompt: "${prompt.substring(0, 50)}..."`
  }

  const res = await fetch(GENERATE_ENDPOINT, {
    method: "POST",
    headers: { "Content-Type": "application/json", "x-goog-api-key": apiKey },
    body: JSON.stringify({ contents: [{ parts: [{ text: prompt }] }] }),
  })

  if (!res.ok) throw new Error(`API error (${res.status}): ${await res.text()}`)

  const b64 = extractContentB64((await res.json()) as Record<string, unknown>)
  if (!b64) throw new Error("No image data returned from Nano Banana model")

  const generationsDir = join(config.outputDir || getDateBasedPath(), "generations")
  const outputPath = await getUniqueFilename(generationsDir, stripImageExtension(config.customName || "generated"), ".png", false)
  const size = await saveB64(b64, outputPath)
  return `Generated image saved: ${outputPath} (${size} bytes)`
}

export async function editImage(imagePath: string, prompt: string, config: ImageConfig = {}): Promise<string> {
  const apiKey = await getGeminiApiKey()

  if (isTestMode()) {
    const editsDir = join(config.outputDir || getDateBasedPath(), "edits")
    const outputPath = await getUniqueFilename(editsDir, stripImageExtension(config.customName || basename(imagePath, extname(imagePath))), ".png", true)
    return `[TEST MODE] Would edit image: ${imagePath} -> ${outputPath} with prompt: "${prompt.substring(0, 50)}..."`
  }

  const { mime, base64 } = await parseImageInput(imagePath)

  const res = await fetch(GENERATE_ENDPOINT, {
    method: "POST",
    headers: { "Content-Type": "application/json", "x-goog-api-key": apiKey },
    body: JSON.stringify({ contents: [{ parts: [{ text: prompt }, { inlineData: { mimeType: mime, data: base64 } }] }] }),
  })

  if (!res.ok) throw new Error(`API error (${res.status}): ${await res.text()}`)

  const b64 = extractContentB64((await res.json()) as Record<string, unknown>)
  if (!b64) throw new Error("No image data returned from Nano Banana model")

  const editsDir = join(config.outputDir || getDateBasedPath(), "edits")
  const outputPath = await getUniqueFilename(editsDir, stripImageExtension(config.customName || basename(imagePath, extname(imagePath))), ".png", true)
  const size = await saveB64(b64, outputPath)
  return `Edited image saved: ${outputPath} (${size} bytes)`
}

export async function analyzeImage(imagePath: string, question: string): Promise<string> {
  const apiKey = await getGeminiApiKey()

  if (isTestMode()) {
    return `[TEST MODE] Would analyze image: ${imagePath} with question: "${question.substring(0, 50)}..." - Mock analysis: This is a test image analysis response.`
  }

  const { mime, base64 } = await parseImageInput(imagePath)

  const res = await fetch(ANALYZE_ENDPOINT, {
    method: "POST",
    headers: { "Content-Type": "application/json", "x-goog-api-key": apiKey },
    body: JSON.stringify({ contents: [{ parts: [{ text: question }, { inlineData: { mimeType: mime, data: base64 } }] }] }),
  })

  if (!res.ok) throw new Error(`API error (${res.status}): ${await res.text()}`)

  const json = (await res.json()) as Record<string, unknown>
  const text = (json?.candidates as Array<{ content?: { parts?: Array<{ text?: string }> } }> | undefined)?.[0]?.content?.parts?.[0]?.text
  if (!text) throw new Error("No analysis returned")

  return text
}