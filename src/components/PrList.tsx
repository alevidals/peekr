import { Fragment, useMemo } from "react";
import { PROVIDERS } from "../providers";
import { prKey, type PullRequest } from "../types";
import { ProviderIcon } from "./Icons";
import { PrItem } from "./PrItem";

interface Props {
  prs: PullRequest[];
  /** Review rows lead with the author; own rows lead with the merge verdict. */
  view: "review" | "mine";
}

interface GroupItem {
  pr: PullRequest;
  /** Position in the whole list; drives the staggered entrance. */
  index: number;
}

interface Group {
  key: string;
  repo: string;
  provider: PullRequest["provider"];
  items: GroupItem[];
}

/**
 * Groups by account + repo, keeping the order in which each repo first appears, and
 * numbers every PR in one pass.
 */
function groupByRepo(prs: PullRequest[]) {
  const groups = new Map<string, Group>();
  const providers = new Set<PullRequest["provider"]>();
  let index = 0;

  for (const pr of prs) {
    providers.add(pr.provider);
    const key = `${pr.account_id}/${pr.repo}`;
    let group = groups.get(key);
    if (!group) {
      group = { key, repo: pr.repo, provider: pr.provider, items: [] };
      groups.set(key, group);
    }
    group.items.push({ pr, index: index++ });
  }

  return { groups: [...groups.values()], showProvider: providers.size > 1 };
}

export function PrList({ prs, view }: Props) {
  const { groups, showProvider } = useMemo(() => groupByRepo(prs), [prs]);

  return (
    <div className="pr-list">
      {groups.map((group) => (
        <div key={group.key} className="pr-list-group">
          <div className="pr-list-group-header">
            <ProviderIcon kind={group.provider} size={12} />
            <span className="pr-list-group-name" title={group.repo}>{group.repo}</span>
            {showProvider && (
              <span className="pr-list-group-provider">{PROVIDERS[group.provider].label}</span>
            )}
            <span className="pr-list-group-count">{group.items.length}</span>
          </div>
          {group.items.map(({ pr, index }, i) => (
            <Fragment key={prKey(pr)}>
              {i > 0 && <div className="pr-item-separator" />}
              <PrItem pr={pr} index={index} view={view} />
            </Fragment>
          ))}
        </div>
      ))}
    </div>
  );
}
