export interface PullRequest {
  id: number;
  title: string;
  author_display_name: string;
  author_avatar_url: string;
  source_branch: string;
  destination_branch: string;
  created_on: string;
  updated_on: string;
  comment_count: number;
  html_url: string;
  repo_slug: string;
  approved: boolean;
  draft: boolean;
}

export interface Config {
  workspace: string;
  repos: string[];
  polling_interval_secs: number;
  has_token: boolean;
}

export interface RepoInfo {
  slug: string;
  name: string;
}
