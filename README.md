# kubectl-split-config

Kubectl plugin which can split a Kubeconfig file into separate files for each context.

Currently the plugin doesn't respect or look at the `$KUBECONFIG` environment
variable, or `~/.kube/config`. Instead users must provide the path to the
kubeconfig file that they want to split on the command line.
