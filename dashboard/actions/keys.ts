"use server"

import prisma from "@/lib/db"
import { auth } from "@/auth"
import { revalidatePath } from "next/cache"
import crypto from "crypto"

export async function createVirtualKey(formData: FormData) {
  const session = await auth()
  if (!session?.user?.id || !session.user.organizationId) throw new Error("Unauthorized")

  if (session.user.role !== "Org_Owner" && session.user.role !== "Org_Admin") {
    throw new Error("Forbidden")
  }

  const name = formData.get("name") as string

  // Generate a random key
  const rawKey = `gw-${crypto.randomBytes(24).toString("hex")}`
  
  // Hash it
  const keyHash = crypto.createHash("sha256").update(rawKey).digest("hex")

  // Prefix
  const prefix = rawKey.substring(0, 7)

  await prisma.virtualKey.create({
    data: {
      organizationId: session.user.organizationId,
      name,
      keyHash,
      prefix
    }
  })

  revalidatePath("/dashboard/keys")
  
  // Return the raw key ONLY ONCE so the user can copy it
  return { success: true, rawKey }
}

export async function revokeVirtualKey(keyId: string) {
  const session = await auth()
  if (!session?.user?.id || !session.user.organizationId) throw new Error("Unauthorized")

  if (session.user.role !== "Org_Owner" && session.user.role !== "Org_Admin") {
    throw new Error("Forbidden")
  }

  await prisma.virtualKey.delete({
    where: {
      id: keyId,
      organizationId: session.user.organizationId
    }
  })

  revalidatePath("/dashboard/keys")
  return { success: true }
}
