import DOMPurify from 'dompurify';
import { marked } from 'marked';

marked.setOptions({
  breaks: true,
  gfm: true
});

export function renderMarkdown(input: string): string {
  const raw = marked.parse(input) as string;
  return DOMPurify.sanitize(raw);
}
