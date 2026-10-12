---
name: run-elsewhere
description: Run a louped experiment on a Slurm cluster (such as ASU Sol), a VM or Google Colab and bring the result back. Use when a model is too large for this machine or the person asks for a real run.
---

1. Run the experiment here first on a small setting (a tiny model or `--limit`). Thus the cluster's
   time goes to the real run.
2. Commit and push the experiment.
   - A project that is a pushed git commit exports as one file, `louped-<id>.sh`. This file clones
     that commit on the cluster.
   - Otherwise the export is a `louped-<id>.tar.gz`, and the note returned with it says why.
   - Colab needs a pushed commit. Without one, the export stops and says why.
3. If louped.toml names the cluster (`[clusters.<name>]`), send it with `submit_job` instead of
   steps 3 and 4. louped copies it there, starts it and brings the result back. Pass `after` (an
   earlier job id on that cluster) to run it after that job succeeds. A gate step is id `gate`
   with options `{"experiment": "<name>"}`. A launch that the gate guards may follow it.
   If `submit_job` is refused with 409, the experiment's gate fails. Do not edit the gate. Tell
   the person which requirement fails and by how much.
   Otherwise, export it with the `export_job` MCP tool. (In the app: Launch, then "Run on" Sol /
   Slurm / VM / Colab, then Export.)
   - Use the provider `colab` for a person without cluster access. Colab takes no GPU fields.
   - Pass the launchable id, the options and a target.
   - Ask for one GPU, unless the model needs more.
   - Ask for the hours the work needs plus 1 hour for the first installs and downloads.
4. You cannot reach the cluster. Tell the person exactly what to run:
   1. Copy the file to the cluster's scratch.
   2. Once per cluster, run `louped cluster token <name>` here, or `hf auth login` on a login
      node (or export `HF_TOKEN`). Gated models need it, and so does pushing to an `hf://`
      remote. If that remote finds no token, the job stops before it runs.
   3. Run `sbatch louped-<id>.sh`, or `bash` on a VM. For a tarball, unpack it and run its
      `job.sh` the same way.
   4. Follow the `.out` file.
   - **For Colab**, tell the person to do these steps:
     1. Upload `louped-<id>.ipynb` to Colab (File > Upload notebook).
     2. Pick a GPU runtime (Runtime > Change runtime type).
     3. For gated models or an `hf://` remote, add the secret `HF_TOKEN` (the key icon), and let
        the notebook read it.
     4. Run all. The output shows as the job runs. Without a remote, the last cell downloads
        `louped-result-<id>.tar.gz`.
5. Bring the result back:
   - **With a remote** (`remote` in louped.toml): the job pushes its own results. Call `pull` (or
     press Pull on Runs). Its runs then read like local ones, marked with the host. Artifact
     files over `large_artifact_mb` (100 MB by default) stay in the remote. They are listed, and
     louped fetches one the first time it is opened.
   - **To set up a remote** (if none is set): ask the person to press Connect on Runs, or to run `louped push` in a
     terminal. Either one sets up a private HF bucket and asks for their token. Never ask for the
     token in chat.
   - **Without a remote**: the person brings back `louped-result-<id>.tar.gz`. Import it with
     `import_result <path>`. For large results, recommend a remote instead.
6. Read the result with `read-results`.
7. Write it up with `write-report`.
