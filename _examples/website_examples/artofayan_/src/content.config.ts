import { defineCollection, z } from "astro:content";

const CATEGORIES = [
  "environments",
  "plein-air",
  "professional",
  "studies",
  "sketches",
] as const;

const projects = defineCollection({
  type: "content",
  schema: z.object({
    title: z.string(),
    category: z.enum(CATEGORIES),
    year: z.number().int(),
    cover: z.string(),
    featured: z.boolean().default(false),
    order: z.number().int().optional(),
    description: z.string().optional(),
    categories: z.array(z.enum(CATEGORIES)).optional(),
    videoUrls: z.array(z.string().url()).optional(),
    coverPosition: z.string().optional(),
    images: z.array(z.string()).optional(),
    client: z.string().optional(),
    printUrl: z.string().url().optional(),
  }),
});

export const collections = {
  projects,
};
