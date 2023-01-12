SELECT
  kube-system,
  testing
FROM
  minikube
WHERE
  pod.status.phase = 'Running'
  OR deployment.metadata.namespace = 'testing'
  OR service.metadata.name = 'hello-minikube'
