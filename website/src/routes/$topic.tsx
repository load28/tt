import { createFileRoute, notFound } from '@tanstack/react-router'
import { isTopic } from '../content'
import { ReleasePage } from '../ui/ReleasePage'
import { ReferencePage, pageHead } from '../ui/ReferencePage'

export const Route = createFileRoute('/$topic')({
  beforeLoad: ({ params }) => {
    if (!isTopic(params.topic) || params.topic === 'overview') throw notFound()
  },
  head: ({ params }) => isTopic(params.topic) ? pageHead('en', params.topic) : {},
  component: TopicPage,
})

function TopicPage() {
  const { topic } = Route.useParams()
  if (!isTopic(topic)) return null
  if (topic === 'release') return <ReleasePage language="en" />
  return <ReferencePage language="en" topic={topic} />
}
