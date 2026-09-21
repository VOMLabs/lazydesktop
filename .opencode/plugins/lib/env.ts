import { readFile } from "fs/promises"
import { resolve } from "path"

/**
 * Configuration for environment variable loading.
 *
 * @remarks
 * Ported from the V1 `.opencode/tool/env` helper so V2 plugins can load
 * API keys from `.env` files without depending on OpenCode 1 APIs.
 */
export interface EnvLoaderConfig {
  /** Custom paths to search for .env files (relative to the current working directory). */
  searchPaths?: string[]
  /** Whether to log when environment variables are loaded. */
  verbose?: boolean
  /** Whether to override existing environment variables. */
  override?: boolean
}

/**
 * Default search paths for .env files.
 */
const DEFAULT_ENV_PATHS = ["./.env", "../.env", "../../.env", "../../../.env"]

/**
 * Load environment variables from .env files.
 * Searches multiple common locations for .env files and loads them into `process.env`.
 */
export async function loadEnvVariables(config: EnvLoaderConfig = {}): Promise<Record<string, string>> {
  const { searchPaths = DEFAULT_ENV_PATHS, verbose = false, override = false } = config

  const loadedVars: Record<string, string> = {}
  const seen = new Set<string>()

  for (const envPath of searchPaths) {
    const fullPath = resolve(envPath)
    if (seen.has(fullPath)) continue
    seen.add(fullPath)

    let content: string
    try {
      content = await readFile(fullPath, "utf8")
    } catch {
      continue // missing .env file is fine
    }

    if (verbose) console.log(`Checking .env file: ${envPath}`)

    for (const rawLine of content.split("\n")) {
      const line = rawLine.trim()
      if (!line || line.startsWith("#") || !line.includes("=")) continue

      const [key, ...valueParts] = line.split("=")
      const value = valueParts.join("=").trim().replace(/^["']|["']$/g, "")

      if (key && value && (override || !process.env[key])) {
        process.env[key] = value
        loadedVars[key] = value
      }
    }
  }

  return loadedVars
}

/**
 * Get a specific environment variable with automatic .env file loading.
 *
 * @returns The environment variable value or `null` if not found.
 */
export async function getEnvVariable(varName: string, config: EnvLoaderConfig = {}): Promise<string | null> {
  let value = process.env[varName]

  if (!value) {
    const loadedVars = await loadEnvVariables(config)
    value = loadedVars[varName] || process.env[varName]
  }

  return value || null
}

/**
 * Get a required environment variable with automatic .env file loading.
 *
 * @throws Error if the variable is not found.
 */
export async function getRequiredEnvVariable(varName: string, config: EnvLoaderConfig = {}): Promise<string> {
  const value = await getEnvVariable(varName, config)

  if (!value) {
    const searchPaths = config.searchPaths || DEFAULT_ENV_PATHS
    throw new Error(
      `${varName} not found. Please set it in your environment or .env file.\n\n` +
        `To fix this:\n` +
        `1. Add to .env: ${varName}=your_value_here\n` +
        `2. Or export it: export ${varName}=your_value_here\n\n` +
        `Current working directory: ${process.cwd()}\n` +
        `Searched paths: ${searchPaths.join(", ")}`,
    )
  }

  return value
}

/**
 * Utility function specifically for API keys.
 *
 * @throws Error if the API key is not found.
 */
export async function getApiKey(apiKeyName: string, config: EnvLoaderConfig = {}): Promise<string> {
  return getRequiredEnvVariable(apiKeyName, config)
}