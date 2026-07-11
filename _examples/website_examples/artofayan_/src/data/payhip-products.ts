export interface StoreProduct {
  slug: string;
  title: string;
  category: string;
  price: string;
  productId: string;
  image: string;
  width: number;
  height: number;
  tagline?: string;
  description?: string;
  paragraphs?: string[];
  includes?: string[];
  topics?: string[];
  format?: string;
  files?: string[];
  gallery?: string[];
}

export const storeProducts: StoreProduct[] = [
  {
    slug: "painterly-brushes",
    title: "Painterly Brush-Set",
    category: "Brush Set",
    tagline: "50+ custom brushes for Procreate 5X",
    price: "$15.00",
    productId: "WCwjY",
    image: "/images/store/painterly-brushes.jpg",
    width: 750,
    height: 421,
    format: "Procreate 5X",
    gallery: [
      "/images/store/painterly-brushes/promo-01.jpg",
      "/images/store/painterly-brushes/promo-02.jpg",
      "/images/store/painterly-brushes/promo-03.jpg",
    ],
    description:
      "Custom brushes made for Procreate 5, inspired by traditional media and tuned for speed and shape exploration.",
    paragraphs: [
      "Some of the main brushes were inspired by traditional medium. All brushes were created using the Procreate 5 engine to simulate a traditional feel while keeping a clear style statement.",
      "I mostly use my iPad for plein airs and to generate ideas for paintings. These brushes were designed to achieve speed while offering a variety of shapes for exploration.",
      "If you have any questions about the brushes, drop me a line - your support is greatly appreciated.",
    ],
    includes: [
      "50+ custom hi-res Procreate 5 brushes",
      "20-minute voice-narrated video on how I use the brushes",
      "PDF listing and explanation",
      "3 full timelapse videos of previous Procreate artworks",
      "High-resolution artwork samples",
      "Free future updates to the brush pack",
    ],
    files: ["MP4 (92MB)", "ZIP (488MB)", "TXT (434B)", "PDF (10MB)", "ZIP (55MB)"],
  },
  {
    slug: "stylized-brushes",
    title: "Stylized Brush-Set",
    category: "Brush Set",
    tagline: "100+ custom brushes for Photoshop",
    price: "$12.50",
    productId: "S0JmF",
    image: "/images/store/stylized-brushes.jpg",
    width: 750,
    height: 421,
    format: "Photoshop CS6+",
    gallery: [
      "/images/store/stylized-brushes/promo-01.jpg",
      "/images/store/stylized-brushes/promo-02.jpg",
      "/images/store/stylized-brushes/promo-03.jpg",
    ],
    description:
      "A set of 100+ custom brushes made for Photoshop, accommodating varied artistic styles from painterly to stylized graphical to realistic painting approaches.",
    paragraphs: [
      "All brushes were constructed using custom-painted stamps and draw inspiration from traditional mediums. They range from painterly to stylized graphical to realistic painting approaches.",
      "If you have any questions about the brushes, drop me a line - your support is greatly appreciated.",
    ],
    includes: [
      "110 custom hi-res Photoshop brushes (CS6 and above)",
      "40 minutes of voice-narrated video demonstrating brush usage",
      "5 high-resolution artworks",
      "5 PSD files containing finished paintings and sketches",
      "Free future brush-pack additions and updates",
    ],
    files: ["ABR (107MB)", "ZIP (500MB)", "MP4 (413MB)"],
  },
  {
    slug: "design-principles",
    title: "Understanding and applying Design Principles!",
    category: "Tutorial",
    price: "$30.00",
    productId: "W97hz",
    image: "/images/store/design-principles.jpg",
    width: 750,
    height: 422,
    format: "Procreate · PDF · Video",
    gallery: [
      "/images/store/design-principles/promo-01.jpg",
      "/images/store/design-principles/promo-02.jpg",
      "/images/store/design-principles/promo-03.jpg",
    ],
    description:
      "A fundamentals-focused tutorial on design principles - how they interact and how to apply them in visual work.",
    paragraphs: [
      "These are the basic principles of art and design. They apply to every aspect of visual design and are not limited by software or medium. Procreate is used for the demonstrations.",
      "If you are looking to improve your design and fundamentals, this is for you.",
    ],
    includes: [
      "2+ hours of voice-narrated analysis and painting videos",
      "30+ page PDF integrating design principles and fundamentals",
      "1 hour 30 minutes of recorded Procreate timelapse",
      "High-res PNG and JPEG exports",
    ],
    topics: ["Design principles", "Fundamentals", "Nuanced application"],
    files: ["MP4 (26MB)", "MP4 (107MB)", "MP4 (200MB)", "MP4 (179MB)", "MP4 (129MB)"],
  },
  {
    slug: "light-age-keyframe",
    title: "Grand Space Opera: Light Age - Keyframe Design - Project files & Walkthrough",
    category: "Keyframes",
    price: "$5.00",
    productId: "k7OLN",
    image: "/images/store/light-age-keyframe.jpg",
    width: 750,
    height: 422,
    format: "PSD · Blender · Video",
    gallery: [
      "/images/store/light-age-keyframe/promo-01.jpg",
      "/images/store/light-age-keyframe/promo-02.jpg",
      "/images/store/light-age-keyframe/promo-03.jpg",
    ],
    description:
      "Project files and walkthrough from the keyframe design challenge - an insight into how I approach a larger project with methods similar to an industry production pipeline.",
    paragraphs: [
      "These will give you an insight on how I approach a larger project. Industry production pipeline follows similar methods.",
      "If you have any questions about the files, drop me a line.",
    ],
    includes: [
      "4 final PSDs with layers intact",
      "2 thumbnail/exploration PSDs with layers intact",
      "1 Blender blockout file",
      "Finals jpegs, thumbnails, and intermediate jpegs",
      "Voice-over video walkthrough of the files and thought process",
      "PureRef reference file",
    ],
    files: [
      "ZIP (24MB)",
      "ZIP (1GB)",
      "ZIP (121MB)",
      "BLEND (1MB)",
      "TXT (2KB)",
      "JPG (1MB)",
      "JPG (3MB)",
      "MP4 (132MB)",
    ],
  },
];

export function getStoreProduct(slug: string): StoreProduct | undefined {
  return storeProducts.find((product) => product.slug === slug);
}

export function getProductHref(product: StoreProduct): string {
  return `/store/${product.slug}`;
}

export function payhipBuyUrl(productId: string): string {
  return `https://payhip.com/buy?link=${productId}`;
}

export function payhipCartUrl(productIds: string[]): string {
  const params = productIds.map((id) => `cart_links[]=${encodeURIComponent(id)}`).join("&");
  return `https://payhip.com/buy?${params}`;
}
