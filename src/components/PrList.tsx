import type { PullRequest } from "../types";
import { PrItem } from "./PrItem";

interface Props {
  prs: PullRequest[];
}

function groupByRepo(prs: PullRequest[]): Record<string, PullRequest[]> {
  return prs.reduce(
    (acc, pr) => {
      (acc[pr.repo_slug] ??= []).push(pr);
      return acc;
    },
    {} as Record<string, PullRequest[]>
  );
}

export function PrList({ prs }: Props) {
  const grouped = groupByRepo(prs);

  return (
    <div className="pr-list">
      {Object.entries(grouped).map(([repo, repoPrs]) => (
        <div key={repo} className="pr-list-group">
          <div className="pr-list-group-header">{repo}</div>
          {repoPrs.map((pr, i) => (
            <>
              {i > 0 && <div className="pr-item-separator" />}
              <PrItem key={pr.id} pr={pr} />
            </>
          ))}
        </div>
      ))}
    </div>
  );
}
