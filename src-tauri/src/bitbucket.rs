use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PullRequest {
    pub id: u64,
    pub title: String,
    pub author_display_name: String,
    pub author_avatar_url: String,
    pub source_branch: String,
    pub destination_branch: String,
    pub created_on: String,
    pub updated_on: String,
    pub comment_count: u32,
    pub html_url: String,
    pub repo_slug: String,
    pub approved: bool,
    pub draft: bool,
}

#[derive(Debug, Deserialize)]
struct BitbucketUser {
    account_id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct UserLinks {
    avatar: Option<Link>,
}

#[derive(Debug, Deserialize)]
struct Link {
    href: String,
}

#[derive(Debug, Deserialize)]
struct Author {
    display_name: Option<String>,
    links: Option<UserLinks>,
}

#[derive(Debug, Deserialize)]
struct BranchRef {
    name: String,
}

#[derive(Debug, Deserialize)]
struct SourceDest {
    branch: BranchRef,
}

#[derive(Debug, Deserialize)]
struct PrLinks {
    html: Link,
}

#[derive(Debug, Deserialize)]
struct Reviewer {
    account_id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Participant {
    role: String,
    #[serde(default)]
    approved: bool,
    user: Reviewer,
}

#[derive(Debug, Deserialize)]
struct RawPullRequest {
    id: u64,
    title: String,
    author: Author,
    source: SourceDest,
    destination: SourceDest,
    created_on: String,
    updated_on: String,
    comment_count: Option<u32>,
    links: PrLinks,
    reviewers: Option<Vec<Reviewer>>,
    participants: Option<Vec<Participant>>,
    #[serde(default)]
    draft: bool,
}

#[derive(Debug, Deserialize)]
pub struct PagedResponse<T> {
    pub values: Vec<T>,
    pub next: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RepoLinks {
    html: Link,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepoInfo {
    pub slug: String,
    pub name: String,
}

#[derive(Debug, Deserialize)]
struct RawRepo {
    slug: String,
    name: String,
}

impl RawPullRequest {
    fn into_pr(self, repo_slug: &str, account_id: &str) -> PullRequest {
        let approved = self
            .participants
            .as_ref()
            .map(|ps| {
                ps.iter().any(|p| {
                    p.approved && p.user.account_id.as_deref() == Some(account_id)
                })
            })
            .unwrap_or(false);

        PullRequest {
            id: self.id,
            title: self.title,
            author_display_name: self
                .author
                .display_name
                .clone()
                .unwrap_or_else(|| "Unknown".to_string()),
            author_avatar_url: self
                .author
                .links
                .and_then(|l| l.avatar)
                .map(|a| a.href)
                .unwrap_or_default(),
            source_branch: self.source.branch.name,
            destination_branch: self.destination.branch.name,
            created_on: self.created_on,
            updated_on: self.updated_on,
            comment_count: self.comment_count.unwrap_or(0),
            html_url: self.links.html.href,
            repo_slug: repo_slug.to_string(),
            approved,
            draft: self.draft,
        }
    }
}

pub async fn fetch_user_account_id(
    client: &reqwest::Client,
    password: &str,
) -> Result<String, String> {
    let resp: BitbucketUser = client
        .get("https://api.bitbucket.org/2.0/user")
        .bearer_auth(password)
        .send()
        .await
        .map_err(|e| e.to_string())?
        .error_for_status()
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())?;

    resp.account_id
        .ok_or_else(|| "Missing account_id in /user response".to_string())
}

pub async fn fetch_workspace_repos(
    client: &reqwest::Client,
    workspace: &str,
    password: &str,
) -> Result<Vec<RepoInfo>, String> {
    let mut repos = Vec::new();
    let mut url = Some(format!(
        "https://api.bitbucket.org/2.0/repositories/{}?pagelen=100",
        workspace
    ));

    while let Some(next_url) = url {
        let resp: PagedResponse<RawRepo> = client
            .get(&next_url)
            .bearer_auth(password)
            .send()
            .await
            .map_err(|e| e.to_string())?
            .error_for_status()
            .map_err(|e| e.to_string())?
            .json()
            .await
            .map_err(|e| e.to_string())?;

        for repo in resp.values {
            repos.push(RepoInfo {
                slug: repo.slug,
                name: repo.name,
            });
        }
        url = resp.next;
    }

    Ok(repos)
}

pub async fn fetch_review_prs_for_repo(
    client: &reqwest::Client,
    workspace: &str,
    repo_slug: &str,
    password: &str,
    account_id: &str,
    use_q_filter: &mut bool,
) -> Result<Vec<PullRequest>, String> {
    let mut prs = Vec::new();

    // +values.participants adds participants to the default response (needed for approved check).
    // draft is included in the default PR response.
    const FIELDS: &str = "%2Bvalues.participants";
    let base_url = if *use_q_filter {
        // state="OPEN" must be inside q — Bitbucket ignores standalone state= when q is present
        format!(
            "https://api.bitbucket.org/2.0/repositories/{}/{}/pullrequests?pagelen=50&fields={}&q=state%3D%22OPEN%22+AND+reviewers.account_id%3D%22{}%22",
            workspace, repo_slug, FIELDS, account_id
        )
    } else {
        format!(
            "https://api.bitbucket.org/2.0/repositories/{}/{}/pullrequests?state=OPEN&pagelen=50&fields={}",
            workspace, repo_slug, FIELDS
        )
    };

    let mut url = Some(base_url);

    while let Some(next_url) = url {
        log::info!("GET {}", next_url);
        let response = client
            .get(&next_url)
            .bearer_auth(password)
            .send()
            .await
            .map_err(|e| e.to_string())?;

        let status = response.status();
        log::info!("  → HTTP {}", status);

        // If q filter returns 400, disable it and retry without
        if *use_q_filter && status == 400 {
            log::info!("  q filter not supported, switching to full fetch + client-side filter");
            *use_q_filter = false;
            let fallback_url = format!(
                "https://api.bitbucket.org/2.0/repositories/{}/{}/pullrequests?state=OPEN&pagelen=50",
                workspace, repo_slug
            );
            url = Some(fallback_url);
            continue;
        }

        let resp: PagedResponse<RawPullRequest> = response
            .error_for_status()
            .map_err(|e| e.to_string())?
            .json()
            .await
            .map_err(|e| e.to_string())?;

        for raw_pr in resp.values {
            let is_reviewer = if *use_q_filter {
                true
            } else {
                raw_pr
                    .participants
                    .as_ref()
                    .map(|ps| {
                        ps.iter().any(|p| {
                            p.role == "REVIEWER"
                                && p.user.account_id.as_deref() == Some(account_id)
                        })
                    })
                    .unwrap_or(false)
            };

            if is_reviewer {
                prs.push(raw_pr.into_pr(repo_slug, account_id));
            }
        }

        url = resp.next;
    }

    Ok(prs)
}

pub async fn fetch_all_review_prs(
    client: &reqwest::Client,
    workspace: &str,
    repos: &[String],
    password: &str,
    account_id: &str,
    use_q_filter: &mut bool,
) -> Result<Vec<PullRequest>, String> {
    let mut all_prs = Vec::new();

    for repo in repos {
        match fetch_review_prs_for_repo(
            client,
            workspace,
            repo,
            password,
            account_id,
            use_q_filter,
        )
        .await
        {
            Ok(prs) => all_prs.extend(prs),
            Err(e) => {
                log::warn!("Failed to fetch PRs for repo {}: {}", repo, e);
            }
        }
    }

    // Sort by created_on descending (newest first)
    all_prs.sort_by(|a, b| b.created_on.cmp(&a.created_on));

    Ok(all_prs)
}
