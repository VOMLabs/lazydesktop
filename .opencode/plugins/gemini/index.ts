import { Plugin } from "@opencode/plugin"
import { join, resolve } from "path"
import { loadEnvVariables } from "../lib/env"
import { analyzeImage, editImage, generateImage, type ImageConfig } from "../lib/gemini"

/**
 * Gemini image tools for OpenCode V2.
 *
 * @remarks
 * Ported from the V1 `.opencode/tool/gemini` tools. V2 registers custom
 * tools through `ctx.tool.transform` instead of exporting `tool()` helpers.
 * The tools are namespaced under `gemini`, so their effective IDs are
 * `gemini_generate`, `gemini_edit`, and `gemini_analyze`.
 */
export default Plugin.define({
  id: "gemini-images",
  async setup(ctx) {
    // Load API keys from the project or working-directory .env before use.
    await loadEnvVariables({
      searchPaths: [join(ctx.location.directory, ".env"), resolve(process.cwd(), ".env")],
      verbose: false,
    })

    const registration = await ctx.tool.transform((editor) => {
      editor.namespace({
        name: "gemini",
        description: "Gemini image generation, editing, and analysis (Nano Banana)",
      })

      editor.add({
        name: "generate",
        description: "Generate an image using Gemini Nano Banana from a text prompt",
        input: {
          type: "object",
          properties: {
            prompt: { type: "string", description: "Text description of the image to generate" },
            outputDir: { type: "string", description: "Custom output directory (default: assets/images/YYYY-MM-DD/)" },
            filename: { type: "string", description: "Custom filename (default: generated)" },
          },
          required: ["prompt"],
          additionalProperties: false,
        },
        options: { namespace: "gemini" },
        execute: async (input) => {
          try {
            const args = input as { prompt: string; outputDir?: string; filename?: string }
            const config: ImageConfig = { outputDir: args.outputDir, customName: args.filename }
            return { content: await generateImage(args.prompt, config) }
          } catch (error) {
            return { content: `Error: ${(error as Error).message}` }
          }
        },
      })

      editor.add({
        name: "edit",
        description: "Edit an existing image using Gemini Nano Banana",
        input: {
          type: "object",
          properties: {
            image: { type: "string", description: "File path or data URL of image to edit" },
            prompt: { type: "string", description: "Edit instruction" },
            outputDir: { type: "string", description: "Custom output directory (default: assets/images/YYYY-MM-DD/)" },
            filename: { type: "string", description: "Custom filename (default: original name with _edit_XXX)" },
          },
          required: ["image", "prompt"],
          additionalProperties: false,
        },
        options: { namespace: "gemini" },
        execute: async (input) => {
          try {
            const args = input as { image: string; prompt: string; outputDir?: string; filename?: string }
            const config: ImageConfig = { outputDir: args.outputDir, customName: args.filename }
            return { content: await editImage(args.image, args.prompt, config) }
          } catch (error) {
            return { content: `Error: ${(error as Error).message}` }
          }
        },
      })

      editor.add({
        name: "analyze",
        description: "Analyze an image using Gemini (text analysis only)",
        input: {
          type: "object",
          properties: {
            image: { type: "string", description: "File path or data URL of image to analyze" },
            question: { type: "string", description: "What to analyze about the image" },
          },
          required: ["image", "question"],
          additionalProperties: false,
        },
        options: { namespace: "gemini" },
        execute: async (input) => {
          try {
            const args = input as { image: string; question: string }
            return { content: await analyzeImage(args.image, args.question) }
          } catch (error) {
            return { content: `Error: ${(error as Error).message}` }
          }
        },
      })
    })

    return () => void registration.dispose()
  },
})