import { createFileRoute } from '@tanstack/react-router'
import { EssayPage, essayHead } from '../ui/EssayPage'

export const Route = createFileRoute('/ko/why')({
  head: () => essayHead('ko'),
  component: () => <EssayPage language="ko" />,
})
