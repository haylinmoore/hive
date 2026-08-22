// Ask a node to deploy a commit, then follow it to a terminal state.
//
// The node does the work: this only posts, polls, and turns the result into
// something readable on the run page.

const POLL_WAIT = 25;
const GIVE_UP_AFTER_ERRORS = 12;

async function request(url, options) {
  const response = await fetch(url, options);
  const text = await response.text();
  let body;
  try {
    body = JSON.parse(text);
  } catch {
    body = { error: text.slice(0, 500) };
  }
  return { status: response.status, body };
}

module.exports = async ({ core, host, sha }) => {
  const base = process.env.HIVED_BASE || `https://hived.${host}.infra.hayl.in`;
  // Audience must match services.hived.audience on the node.
  const token = await core.getIDToken("hived");

  const created = await request(`${base}/v1/deployments`, {
    method: "POST",
    headers: {
      authorization: `Bearer ${token}`,
      "content-type": "application/json",
    },
    body: JSON.stringify({ rev: sha }),
  });

  if (created.status >= 400) {
    core.setFailed(`${host}: ${created.body.error || created.status}`);
    return;
  }

  const id = created.body.id;
  const epoch = created.body.epoch;
  core.info(`${host}: deployment ${id} (${created.body.state})`);

  let deployment = created.body;
  let consecutiveErrors = 0;

  // Activation restarts the listener by design, so a refused connection is
  // expected rather than fatal. Only give up after a run of them.
  while (true) {
    const polled = await request(
      `${base}/v1/deployments/${id}?wait=${POLL_WAIT}`,
    ).catch(() => null);

    if (!polled || polled.status >= 500) {
      if (++consecutiveErrors >= GIVE_UP_AFTER_ERRORS) {
        core.setFailed(`${host}: lost contact while deploying ${sha}`);
        return;
      }
      await new Promise((r) => setTimeout(r, 5000));
      continue;
    }
    consecutiveErrors = 0;

    if (polled.status === 404) {
      core.setFailed(`${host}: deployment ${id} disappeared`);
      return;
    }

    deployment = polled.body;

    // Ids restart at 1 if the node ever loses its state, so make sure we are
    // still watching the commit we asked for.
    if (deployment.epoch !== epoch || deployment.rev !== sha) {
      core.setFailed(`${host}: deployment ${id} is no longer ours`);
      return;
    }

    if (deployment.state !== "queued" && deployment.state !== "running") break;
    core.info(`${host}: ${deployment.state} ${deployment.phase || ""}`);
  }

  await summarise({ core, host, base, id, deployment });
  report({ core, host, deployment });
};

async function summarise({ core, host, base, id, deployment }) {
  let log = "";
  try {
    const response = await fetch(`${base}/v1/deployments/${id}/logs`);
    if (response.ok) log = await response.text();
  } catch {
    log = "(log unavailable)";
  }

  const phases = Object.entries(deployment.durations || {})
    .map(([phase, seconds]) => `${phase} ${Math.round(seconds)}s`)
    .join(", ");

  await core.summary
    .addHeading(`${host}: ${deployment.state}`, 3)
    .addRaw(`\`${deployment.rev.slice(0, 7)}\` ${deployment.subject || ""}<br>`)
    .addRaw(phases ? `${phases}<br>` : "")
    .addDetails(`${host} deploy log`, `<pre>${escapeHtml(log)}</pre>`)
    .write();
}

function escapeHtml(text) {
  return text.replace(
    /[&<>]/g,
    (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;" })[c],
  );
}

function report({ core, host, deployment }) {
  const { state, phase, error } = deployment;

  // Annotations keep only the first line unless newlines are encoded, which is
  // how the whole Nix error quietly turns into one useless line.
  // Degraded says its piece below, with the unit names, so it is skipped here.
  if (error && error.message && state !== "degraded") {
    core.error(error.message.slice(0, 4000).replace(/\n/g, "%0A"), {
      title: `hived: ${state} in ${error.phase} on ${host}`,
    });
  }

  switch (state) {
    case "succeeded":
      core.info(`${host}: deployed`);
      break;
    case "degraded":
      // Yellow, not red. The node is on the new generation either way, and
      // reverting is a decision for a person.
      core.warning(
        `${host}: newly failed units: ${(deployment.new_failed_units || []).join(", ")}`,
        { title: `hived: degraded on ${host}` },
      );
      break;
    case "superseded":
      core.notice(`${host}: a newer commit took the slot`);
      break;
    case "cancelled":
      core.notice(`${host}: cancelled`);
      break;
    default:
      core.setFailed(`${host}: ${state}${phase ? ` in ${phase}` : ""}`);
  }
}

// Exported for testing the state mapping without a live node.
module.exports.report = report;
