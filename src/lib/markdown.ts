import MarkdownIt from 'markdown-it';
import taskLists from 'markdown-it-task-lists';

const md: MarkdownIt = new MarkdownIt({
  html: false,
  linkify: true,
  typographer: false,
  breaks: false
}).use(taskLists, { enabled: false, label: false });

// Force every link to render as a normal anchor; click handler in Preview
// intercepts and routes through the Tauri shell plugin.
md.renderer.rules.link_open = (tokens, idx, options, _env, self) => {
  const token = tokens[idx];
  const href = token.attrGet('href') ?? '';
  token.attrSet('data-external', href);
  token.attrSet('rel', 'noreferrer noopener');
  return self.renderToken(tokens, idx, options);
};

export function renderMarkdown(text: string): string {
  return md.render(text);
}
