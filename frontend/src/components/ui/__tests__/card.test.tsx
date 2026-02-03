import { describe, it, expect } from 'vitest'
import { render, screen } from '@/test/test-utils'
import {
  Card,
  CardHeader,
  CardTitle,
  CardDescription,
  CardContent,
  CardFooter,
  CardAction,
} from '../card'

describe('Card', () => {
  it('renders Card component', () => {
    render(<Card data-testid="card">Content</Card>)
    const card = screen.getByTestId('card')
    expect(card).toBeInTheDocument()
    expect(card).toHaveClass('rounded-xl')
    expect(card).toHaveClass('border')
    expect(card).toHaveClass('bg-card')
  })

  it('renders CardHeader component', () => {
    render(<CardHeader data-testid="header">Header</CardHeader>)
    const header = screen.getByTestId('header')
    expect(header).toBeInTheDocument()
    expect(header).toHaveClass('flex')
    expect(header).toHaveClass('flex-col')
    expect(header).toHaveClass('p-6')
  })

  it('renders CardTitle component', () => {
    render(<CardTitle>Title</CardTitle>)
    const title = screen.getByText('Title')
    expect(title).toBeInTheDocument()
    expect(title).toHaveClass('font-semibold')
  })

  it('renders CardDescription component', () => {
    render(<CardDescription>Description</CardDescription>)
    const description = screen.getByText('Description')
    expect(description).toBeInTheDocument()
    expect(description).toHaveClass('text-muted-foreground')
  })

  it('renders CardContent component', () => {
    render(<CardContent data-testid="content">Content</CardContent>)
    const content = screen.getByTestId('content')
    expect(content).toBeInTheDocument()
    expect(content).toHaveClass('p-6')
    expect(content).toHaveClass('pt-0')
  })

  it('renders CardFooter component', () => {
    render(<CardFooter data-testid="footer">Footer</CardFooter>)
    const footer = screen.getByTestId('footer')
    expect(footer).toBeInTheDocument()
    expect(footer).toHaveClass('flex')
    expect(footer).toHaveClass('items-center')
    expect(footer).toHaveClass('p-6')
  })

  it('renders CardAction component', () => {
    render(<CardAction data-testid="action">Action</CardAction>)
    const action = screen.getByTestId('action')
    expect(action).toBeInTheDocument()
    expect(action).toHaveClass('ml-auto')
    expect(action).toHaveClass('flex')
  })

  it('renders full card structure', () => {
    render(
      <Card data-testid="full-card">
        <CardHeader>
          <CardTitle>Card Title</CardTitle>
          <CardDescription>Card Description</CardDescription>
        </CardHeader>
        <CardContent>Card Content</CardContent>
        <CardFooter>Card Footer</CardFooter>
      </Card>
    )

    expect(screen.getByTestId('full-card')).toBeInTheDocument()
    expect(screen.getByText('Card Title')).toBeInTheDocument()
    expect(screen.getByText('Card Description')).toBeInTheDocument()
    expect(screen.getByText('Card Content')).toBeInTheDocument()
    expect(screen.getByText('Card Footer')).toBeInTheDocument()
  })

  it('accepts custom className for Card', () => {
    render(<Card className="custom-card" data-testid="custom">Custom</Card>)
    const card = screen.getByTestId('custom')
    expect(card).toHaveClass('custom-card')
  })

  it('renders with elevation', () => {
    render(<Card elevation="lg" data-testid="card-lg">Content</Card>)
    expect(screen.getByTestId('card-lg')).toHaveClass('shadow-lg')
  })

  it('renders with hover state', () => {
    render(<Card isHoverable data-testid="card-hover">Content</Card>)
    const card = screen.getByTestId('card-hover')
    expect(card).toHaveClass('hover:shadow-md')
    expect(card).toHaveClass('cursor-pointer')
  })

  it('renders gradient variant', () => {
    render(<Card variant="gradient" data-testid="card-gradient">Content</Card>)
    const card = screen.getByTestId('card-gradient')
    expect(card).toHaveClass('border-primary/20')
    expect(card).toHaveClass('bg-gradient-to-br')
  })
})
