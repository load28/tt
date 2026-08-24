import { createFileRoute } from '@tanstack/react-router'
import { EssayPage, essayHead } from '../ui/EssayPage'

export const Route = createFileRoute('/why')({
  head: () => essayHead('en'),
  component: () => <EssayPage language="en" />,
})
