import { PrismaClient } from '@prisma/client'
import bcrypt from 'bcryptjs'

const prisma = new PrismaClient()

async function main() {
  const passwordHash = await bcrypt.hash('password', 10)

  // Create organization
  const org = await prisma.organization.upsert({
    where: { slug: 'byteeit-admin' },
    update: {},
    create: {
      name: 'ByteeIT Admin',
      slug: 'byteeit-admin',
    },
  })

  // Create user
  const user = await prisma.user.upsert({
    where: { email: 'admin@capllm.test' },
    update: {},
    create: {
      email: 'admin@capllm.test',
      name: 'Alice Owner',
      passwordHash,
      image: 'https://api.dicebear.com/7.x/avataaars/svg?seed=Alice',
      memberships: {
        create: {
          organizationId: org.id,
          role: 'Org_Owner'
        }
      }
    },
  })

  console.log({ org, user })
}

main()
  .then(async () => {
    await prisma.$disconnect()
  })
  .catch(async (e) => {
    console.error(e)
    await prisma.$disconnect()
    process.exit(1)
  })
