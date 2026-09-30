import { Fragment } from "react";
import { PROVIDERS } from "../providers";
import { prKey, type PullRequest } from "../types";
import { ProviderIcon } from "./Icons";
import { PrItem } from "./PrItem";

interface Props {
  prs: PullRequest[];
}

interface Group {
  key: string;
  repo: string;
  provider: PullRequest["provider"];
  prs: PullRequest[];
}

/** Groups by account + repo, keeping the order in which each repo first appears. */
function groupByRepo(prs: PullRequest[]): Group[] {
  const groups = new Map<string, Group>();
  for (const pr of prs) {
    const key = `${pr.account_id}/${pr.repo}`;
    const group = groups.get(key) ?? { key, repo: pr.repo, provider: pr.provider, prs: [] };
    group.prs.push(pr);
    groups.set(key, group);
  }
  return [...groups.values()];
}

export function PrList({ prs }: Props) {
  const groups = groupByRepo(prs);
  // Only worth labelling the provider when more than one is in play.
  const showProvider = new Set(prs.map((pr) => pr.provider)).size > 1;

  let index = 0;
  return (
    <div className="pr-list">
      {groups.map((group) => (
        <div key={group.key} className="pr-list-group">
          <div className="pr-list-group-header">
            <ProviderIcon kind={group.provider} size={12} />
            <span className="pr-list-group-name">{group.repo}</span>
            {showProvider && (
              <span className="pr-list-group-provider">{PROVIDERS[group.provider].label}</span>
            )}
            <span className="pr-list-group-count">{group.prs.length}</span>
          </div>
          {group.prs.map((pr, i) => (
            <Fragment key={prKey(pr)}>
              {i > 0 && <div className="pr-item-separator" />}
              <PrItem pr={pr} index={index++} />
            </Fragment>
          ))}
        </div>
      ))}
    </div>
  );
}
