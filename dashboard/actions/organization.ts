"use server"

import prisma from "@/lib/db"
import { auth } from "@/auth"
import { revalidatePath } from "next/cache"
import bcrypt from "bcryptjs"

export async function inviteMember(formData: FormData) {
  const session = await auth()
  if (!session?.user?.id || !session.user.organizationId) throw new Error("Unauthorized")
  
  // Only Org_Owner and Org_Admin can invite
  if (session.user.role !== "Org_Owner" && session.user.role !== "Org_Admin") {
    throw new Error("Forbidden")
  }

  const email = formData.get("email") as string
  const role = formData.get("role") as string

  // Simple check to see if user exists, if not create dummy one with random password
  let targetUser = await prisma.user.findUnique({ where: { email } })
  if (!targetUser) {
    const passwordHash = await bcrypt.hash(Math.random().toString(36), 10)
    targetUser = await prisma.user.create({
      data: {
        email,
        name: email.split("@")[0],
        passwordHash,
      }
    })
  }

  await prisma.organizationMember.upsert({
    where: {
      userId_organizationId: {
        userId: targetUser.id,
        organizationId: session.user.organizationId
      }
    },
    update: { role },
    create: {
      userId: targetUser.id,
      organizationId: session.user.organizationId,
      role
    }
  })

  revalidatePath("/dashboard/settings/organization")
  return { success: true }
}

export async function revokeAccess(userId: string) {
  const session = await auth()
  if (!session?.user?.id || !session.user.organizationId) throw new Error("Unauthorized")
  
  if (session.user.role !== "Org_Owner" && session.user.role !== "Org_Admin") {
    throw new Error("Forbidden")
  }

  // Cannot revoke oneself if owner
  if (userId === session.user.id && session.user.role === "Org_Owner") {
    throw new Error("Owner cannot revoke themselves")
  }

  await prisma.organizationMember.delete({
    where: {
      userId_organizationId: {
        userId,
        organizationId: session.user.organizationId
      }
    }
  })

  revalidatePath("/dashboard/settings/organization")
  return { success: true }
}
