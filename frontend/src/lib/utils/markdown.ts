import { marked } from 'marked';

marked.setOptions({
  breaks: true,
  gfm: true
});

export function renderMarkdown(input: string): string {
  return marked.parse(input) as string;
}
