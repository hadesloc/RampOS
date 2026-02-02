import { describe, it, expect } from 'vitest'
import { render, screen } from '@/test/test-utils'
import {
  Table,
  TableHeader,
  TableBody,
  TableFooter,
  TableHead,
  TableRow,
  TableCell,
  TableCaption,
} from '../table'

describe('Table', () => {
  it('renders Table component', () => {
    render(
      <Table data-testid="table">
        <TableBody>
          <TableRow>
            <TableCell>Cell</TableCell>
          </TableRow>
        </TableBody>
      </Table>
    )
    const table = screen.getByRole('table')
    expect(table).toBeInTheDocument()
    expect(table).toHaveClass('w-full')
    expect(table).toHaveClass('caption-bottom')
  })

  it('renders TableHeader component', () => {
    render(
      <Table>
        <TableHeader data-testid="header">
          <TableRow>
            <TableHead>Header</TableHead>
          </TableRow>
        </TableHeader>
      </Table>
    )
    const header = screen.getByTestId('header')
    expect(header).toBeInTheDocument()
  })

  it('renders TableBody component', () => {
    render(
      <Table>
        <TableBody data-testid="body">
          <TableRow>
            <TableCell>Cell</TableCell>
          </TableRow>
        </TableBody>
      </Table>
    )
    const body = screen.getByTestId('body')
    expect(body).toBeInTheDocument()
  })

  it('renders TableFooter component', () => {
    render(
      <Table>
        <TableFooter data-testid="footer">
          <TableRow>
            <TableCell>Footer</TableCell>
          </TableRow>
        </TableFooter>
      </Table>
    )
    const footer = screen.getByTestId('footer')
    expect(footer).toBeInTheDocument()
    expect(footer).toHaveClass('border-t')
  })

  it('renders TableHead component', () => {
    render(
      <Table>
        <TableHeader>
          <TableRow>
            <TableHead>Column Header</TableHead>
          </TableRow>
        </TableHeader>
      </Table>
    )
    const head = screen.getByRole('columnheader', { name: 'Column Header' })
    expect(head).toBeInTheDocument()
    expect(head).toHaveClass('font-medium')
    expect(head).toHaveClass('text-muted-foreground')
  })

  it('renders TableRow component', () => {
    render(
      <Table>
        <TableBody>
          <TableRow data-testid="row">
            <TableCell>Cell</TableCell>
          </TableRow>
        </TableBody>
      </Table>
    )
    const row = screen.getByTestId('row')
    expect(row).toBeInTheDocument()
    expect(row).toHaveClass('border-b')
  })

  it('renders TableCell component', () => {
    render(
      <Table>
        <TableBody>
          <TableRow>
            <TableCell>Cell Content</TableCell>
          </TableRow>
        </TableBody>
      </Table>
    )
    const cell = screen.getByRole('cell', { name: 'Cell Content' })
    expect(cell).toBeInTheDocument()
    expect(cell).toHaveClass('p-2')
  })

  it('renders TableCaption component', () => {
    render(
      <Table>
        <TableCaption>Table Caption</TableCaption>
        <TableBody>
          <TableRow>
            <TableCell>Cell</TableCell>
          </TableRow>
        </TableBody>
      </Table>
    )
    const caption = screen.getByText('Table Caption')
    expect(caption).toBeInTheDocument()
    expect(caption).toHaveClass('text-muted-foreground')
  })

  it('renders complete table structure', () => {
    render(
      <Table>
        <TableCaption>A list of users</TableCaption>
        <TableHeader>
          <TableRow>
            <TableHead>Name</TableHead>
            <TableHead>Email</TableHead>
          </TableRow>
        </TableHeader>
        <TableBody>
          <TableRow>
            <TableCell>John Doe</TableCell>
            <TableCell>john@example.com</TableCell>
          </TableRow>
          <TableRow>
            <TableCell>Jane Doe</TableCell>
            <TableCell>jane@example.com</TableCell>
          </TableRow>
        </TableBody>
        <TableFooter>
          <TableRow>
            <TableCell colSpan={2}>Total: 2 users</TableCell>
          </TableRow>
        </TableFooter>
      </Table>
    )

    expect(screen.getByText('A list of users')).toBeInTheDocument()
    expect(screen.getByRole('columnheader', { name: 'Name' })).toBeInTheDocument()
    expect(screen.getByRole('columnheader', { name: 'Email' })).toBeInTheDocument()
    expect(screen.getByText('John Doe')).toBeInTheDocument()
    expect(screen.getByText('jane@example.com')).toBeInTheDocument()
    expect(screen.getByText('Total: 2 users')).toBeInTheDocument()
  })
})
