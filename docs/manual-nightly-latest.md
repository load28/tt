# Temporarily use a Nightly for default npm installs

During early development, a maintainer may explicitly point npm `latest` at an
already published main Nightly. This does not make the version stable. It does
not rebuild packages, publish a new version, change `next`, move GitHub Release
tags, or change the release branch.

## Run the manual action

1. Wait for the scheduled or manually dispatched main CI and its Nightly
   publication to finish successfully.
2. Open **Actions → Temporarily Promote Nightly to Latest → Run workflow** and
   choose **main**. There are no version or npm-tag inputs.
3. Review the verification job summary: it links the successful CI and lists
   the source commit, all target versions, and previous `latest` tags.
4. Approve the `production` deployment. The action revalidates the frozen
   candidate and existing registry packages before updating tags.

The equivalent dispatch is:

```sh
gh workflow run promote-nightly-latest.yml --ref main
```

The action selects the most recent successful scheduled/manual main CI among
100 recent successful runs. Its metadata must still be available, and every
package must already exist at the expected version under `next`. If publication
is pending, a newer Nightly replaced `next`, or metadata has expired, the action
fails before changing any tag; wait for the new Nightly and dispatch again.
It does not silently choose an older build or follow a changed candidate after
approval.

The five compiler platform packages, `@openload28/tt-lang`,
`@openload28/create-tt`, and `@openload28/unplugin-tt` are included. The unplugin
keeps the independent version recorded in the Nightly metadata. The launcher
must depend on the matching versions of all five platform packages.

## Temporary policy and recovery

Only an explicit dispatch runs this action. Future main pushes and Nightly
publications continue updating `next`; they do not automatically refresh
`latest`. Default installs receive the chosen Nightly until the next manual
promotion or a normal stable release updates `latest`. Existing lockfiles and
installed packages are not automatically upgraded.

npm has no atomic multi-package tag update. All packages are preflighted before
the first write, and normal publishing shares the same concurrency group. If
npm fails midway, rerun the failed job while the candidate is still `next`;
completed tag updates are skipped. Successful writes are verified with up to six
online-preferring reads of mismatched packages. Shared backoff rounds wait 2, 4,
8, 16, and 30 seconds (60 seconds of waiting plus request time); they never
repeat tag writes. Persistent mismatches fail with expected and observed tags.
The summary records previous tags for manual
rollback with `npm dist-tag add PACKAGE@PREVIOUS_VERSION latest` (or
`npm dist-tag rm PACKAGE latest` if the tag was previously absent). Registry
administrators making tag changes outside these workflows are not covered by
the concurrency group.

To stop this temporary policy, stop dispatching the action and publish the
normal stable release. Remove this workflow when the early-development policy
is no longer needed. No automatic release-version rule needs to change.
